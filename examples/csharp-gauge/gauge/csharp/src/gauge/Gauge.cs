using Xirang;

namespace Gauge;

/// <summary>
/// 一个用 C# 特性声明的注册面：特性是构建期载体，Roslyn 生成器把它写成 xirang.faces 记录。
/// A registration face declared with the C# attribute: the attribute is a build-time carrier, and
/// the Roslyn generator writes it as an xirang.faces record.
/// </summary>
[Soil(Parent = XirangFaces.PanelNodeId, Named = "Gauge", Exports = new[] { "gauge" }, NameEn = "Gauge")]
public sealed class Gauge
{
    /// <summary>
    /// 一个普通的业务方法，证明声明不改变类本身。
    /// An ordinary business method, proving the declaration does not change the class.
    /// </summary>
    /// <returns>the reading</returns>
    public static int Reading() => 42;
}
