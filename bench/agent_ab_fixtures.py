"""Executable, native-registration repair tasks and withheld behavioral oracles."""
from pathlib import Path
import shutil
import subprocess


CASES = {
    "floor": {
        "contract": "Return max(amount, floor), for every i64 amount and floor.",
        "broken": "if a > b { b } else { a }",
        "fixed": "a.max(b)", "example": (1, 2, 2), "depth": 8,
        "oracle": "a.max(b)",
    },
    "batches": {
        "contract": "Return ceil(amount / capacity). amount >= 0 and capacity > 0. Support the entire nonnegative i64 range without overflow.",
        "broken": "a / b",
        "fixed": "a / b + i64::from(a % b != 0)", "example": (7, 3, 3), "depth": 12,
        "oracle": "((i128::from(a) + i128::from(b) - 1) / i128::from(b)) as i64",
    },
    "bucket": {
        "contract": "Return the mathematical floor of coordinate / bucket_width. coordinate is any i64 and bucket_width > 0. Negative coordinates must map to the lower bucket.",
        "broken": "a / b",
        "fixed": "a.div_euclid(b)", "example": (-1, 8, -1), "depth": 16,
        "oracle": "a.div_euclid(b)",
    },
}


def write(path, body):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def create(root, case, scaffold, repo):
    """Build the same registered tree for either arm; tests expose one symptom only."""
    shutil.copytree(scaffold, root)
    lib = root / "src/lib.rs"
    lib.write_text(lib.read_text() + "\npub use provider::object::{consumer, sibling};\n")
    case_data = CASES[case]
    manifest = root / "Cargo.toml"
    manifest.write_text(manifest.read_text().replace('[workspace]', '[workspace]\nmembers = ["arithmetic"]') +
                        '\n[dependencies.arithmetic]\npath = "arithmetic"\n')
    write(root / "arithmetic/Cargo.toml", '[package]\nname="arithmetic"\nversion="0.1.0"\nedition="2024"\n')
    write(root / "arithmetic/src/lib.rs", "/// " + case_data["contract"] +
          "\npub fn compute(a: i64, b: i64) -> i64 { " + case_data["broken"] + " }\n")
    provider = root / "src/provider/provider.rs"
    provider.write_text(provider.read_text() + "\n/// Shared provider; preserve delegation to the arithmetic crate.\n"
                        "pub fn compute(a: i64, b: i64) -> i64 { arithmetic::compute(a, b) }\n")
    for name in ("consumer", "sibling"):
        path = root / f"src/provider/object/{name}/{name}.rs"
        body = path.read_text() + "\n/// Public consumer; forwards both arguments unchanged.\n"
        body += "pub fn render(a: i64, b: i64) -> i64 { hop0(a, b) }\n"
        for i in range(case_data["depth"]):
            target = f"hop{i+1}" if i + 1 < case_data["depth"] else "crate::provider::compute"
            body += f"fn hop{i}(a: i64, b: i64) -> i64 {{ {target}(a, b) }}\n"
        path.write_text(body)
    a, b, value = case_data["example"]
    write(root / "tests/symptom.rs", f"#[test]\nfn visible_symptom() {{ assert_eq!(repair_fixture::consumer::render({a}, {b}), {value}); }}\n")
    write(root / "TASK.md", f"Repair the failing visible_symptom test by fixing the shared behavior.\n\n"
          f"Contract: {case_data['contract']}\n\n"
          "Both public consumers must continue delegating to the provider and arithmetic::compute, "
          "passing their arguments unchanged. Preserve public signatures, registration declarations, "
          "crate boundaries, and existing tests. Add regression tests where useful. "
          "Do not hard-code observed outputs or copy the calculation into consumers.\n")


def hidden_tests(case, replacement=False):
    spec = CASES[case]
    values = "[i64::MIN, -1001, -17, -1, 0, 1, 2, 17, 1001, i64::MAX]"
    if case == "batches":
        values = "[0, 1, 2, 17, 1001, i64::MAX - 1, i64::MAX]"
    divisor = "[-99, -1, 0, 1, 7, i64::MAX]" if case == "floor" else "[1, 2, 7, 99, i64::MAX]"
    oracle = "a.wrapping_mul(13).wrapping_add(b.wrapping_mul(7))" if replacement else spec["oracle"]
    return f'''#[test]
fn hidden_all_consumers_and_boundaries() {{
    for a in {values} {{ for b in {divisor} {{
        let expected = {oracle};
        assert_eq!(arithmetic::compute(a,b), expected, "provider {{a}} {{b}}");
        assert_eq!(repair_fixture::consumer::render(a,b), expected, "consumer {{a}} {{b}}");
        assert_eq!(repair_fixture::sibling::render(a,b), expected, "sibling {{a}} {{b}}");
    }} }}
}}
'''


def check(tree, case, output, *, replacement=False):
    """Score a copy; never expose hidden tests in the agent's writable tree."""
    output.mkdir(parents=True, exist_ok=False)
    clean = output / "tree"
    shutil.copytree(tree, clean, ignore=shutil.ignore_patterns('target', '.codegraph', '.git'))
    write(clean / "tests/hidden.rs", hidden_tests(case, replacement))
    if replacement:
        write(clean / "arithmetic/src/lib.rs", "pub fn compute(a: i64, b: i64) -> i64 { a.wrapping_mul(13).wrapping_add(b.wrapping_mul(7)) }\n")
    # Copies preserve mtimes: clear just the adjudicated packages before reusing
    # dependency artifacts. Otherwise Cargo can reuse a different copy's tests.
    for profile in ('dev', 'release'):
        cleared = subprocess.run(['cargo', 'clean', '--offline', '--profile', profile,
                                  '-p', 'arithmetic', '-p', 'repair_fixture'],
                                 cwd=clean, capture_output=True, text=True, check=True, timeout=60)
        (output / f'clean-{profile}.log').write_text(cleared.stdout + cleared.stderr)
    commands = [["cargo", "test", "--offline", "--test", "hidden"]] if replacement else [
        ["cargo", "test", "--offline", "--workspace"],
        ["cargo", "test", "--offline", "--workspace", "--release"],
    ]
    results = []
    for i, command in enumerate(commands):
        run = subprocess.run(command, cwd=clean, capture_output=True, text=True, timeout=300)
        log = run.stdout + run.stderr
        (output / f"cargo-{i}.log").write_text(log)
        results.append({"command": command, "exit_code": run.returncode,
                        "passed": run.returncode == 0 and 'hidden_all_consumers_and_boundaries ... ok' in log,
                        "assertion_failed": 'hidden_all_consumers_and_boundaries ... FAILED' in log})
    return results
