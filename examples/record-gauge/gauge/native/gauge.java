// 这条声明的真实 Java 形态。本检出的记录由 `native/write_faces.py` 生成，因为 JVM 工具链还没有接上；
// 记录文件里的字节就是下面这条声明会产出的那条（kind/父/导出/源码拼写）。
// The real Java spelling of this declaration. This checkout's record is produced by
// `native/write_faces.py` because the JVM toolchain is not wired yet; the record's bytes are the ones
// the declaration below would produce (kind, parent, exports, source spelling).

import xirang.faces;
import xirang.soil.Soil;

@Soil(under = faces.root.NODE_ID, named = "Gauge", exports = {"gauge"})
public final class Gauge {
    private Gauge() {}
}
