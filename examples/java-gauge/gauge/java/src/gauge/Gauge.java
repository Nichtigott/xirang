package gauge;

import xirang.Faces;
import xirang.Soil;

/**
 * 一个用 Java 注解声明的注册面：注解是 SOURCE 保留的，构建期处理器把它写成 xirang.faces 记录。
 * A registration face declared with the Java annotation: the annotation is SOURCE-retained, and the
 * build-time processor writes it as an xirang.faces record.
 */
@Soil(parent = Faces.PANEL_NODE_ID, named = "Gauge", exports = {"gauge"}, name_en = "Gauge")
public final class Gauge {
    private Gauge() {}

    /**
     * 一个普通的业务方法，证明声明不改变类本身。
     * An ordinary business method, proving the declaration does not change the class.
     *
     * @return the reading
     */
    public static int reading() {
        return 42;
    }
}
