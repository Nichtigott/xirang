/**
 * The JavaScript side of a XiRang registration face.
 * XiRang 注册面的 JavaScript 一侧。
 *
 * A face written in JavaScript declares itself with one call at module top level:
 * 用 JavaScript 写的面在模块顶层用一次调用声明自己：
 *
 *     import { faces, soil } from "xirang";
 *
 *     export class Gauge {}
 *
 *     soil.face({
 *         under: faces.shop.panel.NODE_ID,
 *         named: "Gauge",
 *         exports: ["gauge"],
 *         needs_registry: false,
 *     });
 *
 * Importing the module runs the call, so the declaration is registered at import time; the build
 * reads the same call statically and never executes the module's business code. `faces` is the
 * generated object directory; before a build writes one it is a fallback proxy, so the same source
 * imports and type-checks in an editor.
 * 导入模块会执行这次调用，因此声明在导入期被登记；构建静态读取同一次调用，从不执行模块的业务代码。
 * `faces` 是生成的对象目录；在构建写出它之前它是一个兜底代理，因此同一份源码在编辑器里能导入并通过类型检查。
 *
 * The SDK does not transform anything: the call records the field object and returns it. Validation
 * is the build's job, and the build reads the source, not this process.
 * SDK 不做任何变换：调用记录字段对象并原样返回。校验是构建的职责，而构建读的是源码，不是本进程。
 */

/**
 * Every face this process has declared, in import order.
 * 本进程声明过的每个面，按导入顺序。
 */
const DECLARED = [];

/**
 * Declare one face.
 * 声明一个面。
 *
 * @param {object} fields the registration fields, with `under` and `named` required by the build.
 * @returns {object} the same object, so a caller can keep using it.
 */
function face(fields) {
    if (fields === null || typeof fields !== "object" || Array.isArray(fields)) {
        throw new TypeError("soil.face(...) takes one object of registration fields");
    }
    DECLARED.push(fields);
    return fields;
}

/**
 * The decorator-like namespace a face declaration uses.
 * 面声明使用的、类似装饰器的命名空间。
 */
export const soil = Object.freeze({ face });

/**
 * The declarations this process recorded, in import order.
 * 本进程记录下来的声明，按导入顺序。
 *
 * Each entry is a shallow copy: a caller cannot mutate the collector by editing what it reads.
 * 每条是浅拷贝：调用方不能通过改写读到的东西来改动收集器。
 *
 * @returns {object[]} one entry per `soil.face(…)` call that ran.
 */
export function declarations() {
    return DECLARED.map((fields) => ({ ...fields }));
}

/**
 * The face the module declared under one name, when it declared one.
 * 模块以某个名字声明的面（若声明了）。
 *
 * @param {string} name the `named:` value.
 * @returns {object|undefined} a copy of that declaration, or `undefined`.
 */
export function declared(name) {
    const found = DECLARED.find((fields) => fields.named === name);
    return found === undefined ? undefined : { ...found };
}

/**
 * A generated object directory: attribute access walks the logical tree.
 * 生成的对象目录：属性访问沿逻辑树行走。
 *
 * The real module is written by the build with one attribute per face path; this fallback exists
 * so a source file imports and type-checks before the first build. `NODE_ID` is the leaf: it
 * carries the logical path the build resolves, and the same path the static reader checks.
 * 真实模块由构建写出，每个面路径一个属性；这个兜底让源码在第一次构建之前也能导入与类型检查。`NODE_ID`
 * 是叶子：它携带构建要解析的逻辑路径，也是静态读取器校验的同一个路径。
 *
 * @param {string[]} path the segments walked so far.
 * @returns {object} a directory whose children are more directories, or a `NODE_ID` leaf.
 */
function directory(path) {
    return new Proxy(
        {},
        {
            get(_target, property) {
                if (typeof property !== "string") {
                    return undefined;
                }
                if (property === "NODE_ID") {
                    return Object.freeze({
                        NODE_ID: path.length === 0 ? "root" : path.join("::"),
                    });
                }
                return directory([...path, property]);
            },
        },
    );
}

/**
 * The generated object directory, as a fallback proxy before a build writes the real one.
 * 生成的对象目录，在构建写出真实模块之前以兜底代理的形式存在。
 */
export const faces = directory([]);
