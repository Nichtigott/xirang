// 导入这个模块就会登记它声明的面；这些断言运行在 Node 的 strip-only TypeScript 下。
// Importing the module registers the face it declares; these assertions run under Node's
// strip-only TypeScript, so the file is executed the way the production pipeline runs `.ts` units.
import { declarations } from "xirang";
import { Gauge } from "../src/gauge.ts";

import assert from "node:assert/strict";
import test from "node:test";

test("importing the module registers the typed face", () => {
    const faces = declarations();
    assert.equal(faces.length, 1, "one face is registered");
    assert.equal(faces[0].named, "Gauge");
    assert.deepEqual(faces[0].exports, ["gauge"]);
    assert.equal(faces[0].needs_registry, false);
});

test("the declaration does not change the business class", () => {
    const gauge: Gauge = new Gauge();
    assert.equal(gauge.reading(), 42);
});

test("the generated object directory names the same logical path the declaration uses", async () => {
    const { faces } = await import("xirang");
    assert.equal(faces.shop.panel.NODE_ID.NODE_ID, "shop::panel");
});
