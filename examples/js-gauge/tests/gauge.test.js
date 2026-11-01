// 导入这个模块就会登记它声明的面；下面断言的是运行期真的能看见那条声明。
// Importing the module registers the face it declares; the test asserts the declaration is really
// visible at run time.
import { declarations } from "xirang";
import { Gauge } from "../src/gauge.js";

import assert from "node:assert/strict";
import test from "node:test";

test("importing the module registers the face", () => {
    const faces = declarations();
    assert.equal(faces.length, 1, "one face is registered");
    assert.equal(faces[0].named, "Gauge");
    assert.deepEqual(faces[0].exports, ["gauge"]);
    assert.equal(faces[0].needs_registry, false);
    assert.equal(faces[0].name_en, "Gauge");
});

test("the declaration does not change the business class", () => {
    assert.equal(new Gauge().reading(), 42);
});

test("the generated object directory names the same logical path the declaration uses", async () => {
    const { faces } = await import("xirang");
    assert.equal(faces.root.NODE_ID.NODE_ID, "root");
    assert.equal(faces.shop.panel.NODE_ID.NODE_ID, "shop::panel");
});
