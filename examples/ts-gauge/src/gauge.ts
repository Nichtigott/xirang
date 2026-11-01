// 一个在 TypeScript 里声明的注册面：导入这个模块就会执行声明，类型标注由 SDK 的 `index.d.ts` 校验。
// A registration face declared in TypeScript: importing this module runs the declaration, and the
// declaration object is type-checked against the SDK's `index.d.ts`.
import { faces, soil } from "xirang";

/**
 * 这个面命名的那门业务类型。
 * The business type this registration face is named after.
 */
export class Gauge {
    /**
     * 一个普通的业务方法，证明声明不影响类本身。
     * An ordinary business method, proving the declaration does not change the class.
     *
     * @returns {number} the reading.
     */
    reading(): number {
        return 42;
    }
}

soil.face({
    under: faces.root.NODE_ID,
    named: "Gauge",
    exports: ["gauge"],
    name_en: "Gauge",
    needs_registry: false,
});
