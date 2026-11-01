//! Pins for the C# adapter: generation, reading, precise rewrite, and a real `dotnet build`.
//! C# 适配器的钉子：生成、读取、精确改写，以及一次真实 `dotnet build`。

use super::*;
use xirang_kernel::authoring::{
    FaceAuthoringRequest, ParentReference, RegistrationUpdate, SourceContext,
};
use xirang_kernel::interchange::LanguageId;

fn csharp() -> LanguageId {
    LanguageId::parse_language("csharp").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: csharp(),
        operation,
        destination: "csharp/src/gauge/gauge.cs".to_owned(),
        parent: ParentReference::Root,
        registration: RegistrationUpdate::new(),
        source_context: SourceContext::new(),
    }
}

fn with(registration: RegistrationUpdate) -> FaceAuthoringRequest {
    let mut request = request(AuthoringOperation::Create);
    request.registration = registration;
    request
}

fn field(registration: &mut RegistrationUpdate, field: RegistrationField, text: &str) {
    registration
        .set_text(field, FieldUpdate::Set(text))
        .expect("the fixture value is valid");
}

/// A create writes the attribute spelling: using, namespace, class, and the argument list.
/// 新建写出特性拼法：using、命名空间、类与实参表。
#[test]
fn a_create_writes_the_attribute_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = CSharpLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(source.contains("using Xirang;\n"), "{source}");
    assert!(source.contains("namespace Gauge;\n"), "{source}");
    assert!(source.contains("[Soil(\n"), "{source}");
    assert!(
        source.contains("    Parent = XirangFaces.RootNodeId,\n"),
        "{source}"
    );
    assert!(source.contains("    Named = \"Gauge\",\n"), "{source}");
    assert!(
        source.contains("    Exports = new[] { \"gauge\" }\n"),
        "{source}"
    );
    assert!(
        source.contains("public sealed class Gauge\n{\n}\n"),
        "{source}"
    );
}

/// The generated declaration reads back to the same normalized registration.
/// 生成的声明能读回同一份规范注册。
#[test]
fn a_generated_declaration_reads_back_to_the_same_fields() {
    let mut registration = RegistrationUpdate::new();
    for (field_, text) in [
        (RegistrationField::Kind, "Gauge"),
        (RegistrationField::StableName, "stable_gauge"),
        (RegistrationField::Exports, "gauge,reading"),
        (RegistrationField::Provides, "geometry"),
        (RegistrationField::Requires, "clock=>SystemClock"),
        (RegistrationField::Preset, "gauge.preset"),
        (RegistrationField::Parts, "gauge.parts"),
        (RegistrationField::NameZh, "仪表"),
        (RegistrationField::NameEn, "Gauge"),
        (RegistrationField::SummaryZh, "一个仪表"),
        (RegistrationField::SummaryEn, "one gauge"),
        (RegistrationField::NeedsRegistry, "true"),
    ] {
        field(&mut registration, field_, text);
    }
    let plan = CSharpLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let declaration =
        read_declaration(&plan.files[0].updated).expect("the generated source reads back");
    assert_eq!(declaration.kind(), "Gauge");
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{}\n{:?}",
        plan.files[0].updated,
        declaration.face.registration
    );
}

/// A modify replaces only the argument list: using, namespace, class body and trailing code survive.
/// 修改只替换实参表：using、命名空间、类体与后面的代码逐字保留。
#[test]
fn a_modify_touches_only_the_argument_list() {
    let source = "\
using Xirang;

namespace Gauge;

[Soil(Parent = XirangFaces.RootNodeId, Named = \"Gauge\", Exports = new[] { \"gauge\" })]
public sealed class Gauge
{
    /// <summary>A business method that must survive an edit.</summary>
    public static int Reading() => 42;
}

// A trailing comment.
";
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::Exports,
        "gauge,reading",
    );
    let mut update = request(AuthoringOperation::Modify);
    update.registration = registration;
    update.source_context = SourceContext::new().with_existing_source(source);
    let plan = CSharpLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.contains("public static int Reading() => 42;\n}"),
        "the class body survives:\n{updated}"
    );
    assert!(
        updated.contains("    Exports = new[] { \"gauge\", \"reading\" }\n"),
        "the named argument is rewritten normalized:\n{updated}"
    );
    assert!(
        updated.ends_with("// A trailing comment.\n"),
        "everything after the declaration survives:\n{updated}"
    );
    assert_eq!(plan.files[0].edits.len(), 1, "one precise edit");
}

/// A parent is named by the generated constant for its module, the same rule the build writes.
/// 父级用其模块的生成常量点名，与构建写出的规则相同。
#[test]
fn a_parent_is_named_by_the_generated_face_constant() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: "shop::panel".to_owned(),
    };
    let plan = CSharpLanguage
        .plan(&request)
        .expect("the parent is nameable");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("    Parent = XirangFaces.ShopPanelNodeId,\n"),
        "{source}"
    );
    assert!(
        plan.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("`[record] language = \"csharp\"`")),
        "the plan says which record table key makes the constant exist: {:?}",
        plan.diagnostics
    );
}

/// An empty parent module is refused by name.
/// 空的父模块按名拒绝。
#[test]
fn an_empty_parent_module_is_refused() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: String::new(),
    };
    let error = CSharpLanguage
        .plan(&request)
        .expect_err("an empty module names no constant");
    assert!(error.contains("empty parent module"), "{error}");
}

/// Changing the kind in place is a rename, not a field edit, so it is refused.
/// 原地改 kind 是重命名而不是字段编辑，因此拒绝。
#[test]
fn changing_the_kind_in_place_is_refused() {
    let source = "[Soil(Parent = XirangFaces.RootNodeId, Named = \"Gauge\")]\npublic sealed class Gauge {}\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Dial");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context = SourceContext::new().with_existing_source(source);
    let error = CSharpLanguage
        .plan(&request)
        .expect_err("a kind change is a rename");
    assert!(error.contains("renames the class"), "{error}");
}

/// The committed example reads back to the declaration its runtime test asserts.
/// 入库的示例读回成它运行期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/csharp-gauge/gauge");
    let source =
        std::fs::read_to_string(root.join("csharp/src/gauge/Gauge.cs")).expect("the example");
    let declaration = read_declaration(&source).expect("the example's declaration");
    assert_eq!(declaration.kind(), "Gauge");
    assert_eq!(declaration.face.parent, "XirangFaces.PanelNodeId");
}

/// The .NET toolchain, when this machine has one.
/// 本机有 .NET 工具链时返回它。
fn dotnet() -> Option<String> {
    if let Ok(from_env) = std::env::var("XIRANG_DOTNET")
        && std::process::Command::new(&from_env)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    {
        return Some(from_env);
    }
    std::process::Command::new("dotnet")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "dotnet".to_owned())
}

/// The generated source really compiles against the SDK and the generated constants (skipped
/// without the .NET SDK).
/// 生成的源码真的能在 SDK 与生成的常量下编译（没有 .NET SDK 时跳过）。
#[test]
fn a_generated_declaration_compiles_against_the_sdk() {
    let Some(dotnet) = dotnet() else {
        eprintln!("no .NET SDK (XIRANG_DOTNET or `dotnet`); the generated source was not compiled");
        return;
    };
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let mut request = with(registration);
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: "panel".to_owned(),
    };
    let plan = CSharpLanguage.plan(&request).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-csharp-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    let source = directory.join("src/gauge");
    std::fs::create_dir_all(&source).expect("the fixture directory");
    std::fs::write(source.join("Gauge.cs"), &plan.files[0].updated).expect("the generated source");
    std::fs::write(
        source.join("Faces.cs"),
        "// Code generated by the build; do not edit.\ninternal static class XirangFaces\n{\n    internal const string RootNodeId = \"d50df7e06896ebbf0b0912f3603dbc67\";\n    internal const string PanelNodeId = \"cff196f1773f4ab1f1c49314821f99ea\";\n}\n",
    )
    .expect("the generated constants");
    std::fs::write(
        directory.join("Authoring.csproj"),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net10.0</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"{}\" />\n  </ItemGroup>\n</Project>\n",
            crate::sdk::csharp_include_dir().join("Xirang.Sdk/Xirang.Sdk.csproj").display()
        ),
    )
    .expect("the project file");
    let home = directory.join(".dotnet-home");
    let packages = directory.join(".nuget");
    let output = std::process::Command::new(dotnet)
        .arg("build")
        .arg(directory.join("Authoring.csproj"))
        .arg("-v")
        .arg("q")
        .env("DOTNET_CLI_HOME", &home)
        .env("NUGET_PACKAGES", &packages)
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .output()
        .expect("dotnet runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "dotnet refused the generated source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
