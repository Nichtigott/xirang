// 一个在 JavaScript 里声明的注册面：导入这个模块就会执行声明。
// A registration face declared in JavaScript: importing this module runs the declaration.
//
// `faces` 是生成的对象目录的兜底代理；构建写出真实模块后它解析成同一批常量。
// `faces` is the fallback proxy for the generated object directory; once the build writes the real
// module it resolves to the same constants.
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
    reading() {
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
