namespace Xirang;

/// <summary>
/// One XiRang registration face, declared on a C# class.
/// 一个声明在 C# 类上的 XiRang 注册面。
/// </summary>
/// <remarks>
/// The attribute is a build-time carrier, not a runtime transformation: the build's
/// <c>xirang-faces</c> generator parses the source with Roslyn and writes the same
/// 1,228-byte version-3 records the C, C++, Zig, Java and Go SDKs produce. <see cref="Parent"/> is
/// the generated constant's value, not a spelling the generator resolves: the build writes a
/// generated <c>Faces.cs</c> whose <c>RootNodeId</c> is a hex string, and the declaration writes
/// <c>[Soil(Parent = XirangFaces.RootNodeId, Named = "Gauge")]</c>. A wrong parent is refused when
/// the tree is assembled, by name.
/// 注解是构建期载体而不是运行时变换：构建的 <c>xirang-faces</c> 生成器用 Roslyn 解析源码，写出与
/// C、C++、Zig、Java、Go SDK 相同的 1,228 字节版本 3 记录。<see cref="Parent"/> 是生成常量的值，
/// 不是生成器要解析的拼写：构建写出一个生成的 <c>Faces.cs</c>，其中的 <c>RootNodeId</c> 是十六进制
/// 字符串，声明写成 <c>[Soil(Parent = XirangFaces.RootNodeId, Named = "Gauge")]</c>。父级写错会在
/// 组装树时按名拒绝。
/// </remarks>
[AttributeUsage(AttributeTargets.Class)]
public sealed class SoilAttribute : Attribute
{
    /// <summary>The parent identity, as 32 lowercase hex characters.</summary>
    /// <summary>父身份，32 个小写十六进制字符。</summary>
    public required string Parent { get; init; }

    /// <summary>The display name, which is the kind the registry keys on.</summary>
    /// <summary>显示名，也就是注册机据以索引的 kind。</summary>
    public required string Named { get; init; }

    /// <summary>The registry key, when it differs from <see cref="Named"/>.</summary>
    /// <summary>注册机索引本面所用的名字（当它与 <see cref="Named"/> 不同时）。</summary>
    public string RegistryName { get; init; } = "";

    /// <summary>The author-owned identity that survives a source move.</summary>
    /// <summary>作者拥有的、可跨源码移动保持的身份名。</summary>
    public string StableName { get; init; } = "";

    /// <summary>The export names.</summary>
    /// <summary>导出名称。</summary>
    public string[] Exports { get; init; } = [];

    /// <summary>The capability names this face provides.</summary>
    /// <summary>本面提供的能力名。</summary>
    public string[] Provides { get; init; } = [];

    /// <summary>The <c>capability=&gt;provider</c> requirements.</summary>
    /// <summary><c>capability=&gt;provider</c> 形式的能力需求。</summary>
    public string[] Requires { get; init; } = [];

    /// <summary>The preset type path.</summary>
    /// <summary>preset 类型路径。</summary>
    public string Preset { get; init; } = "";

    /// <summary>The parts type path.</summary>
    /// <summary>parts 类型路径。</summary>
    public string Parts { get; init; } = "";

    /// <summary>The Chinese half of the display name.</summary>
    /// <summary>显示名的中文部分。</summary>
    public string NameZh { get; init; } = "";

    /// <summary>The English half of the display name.</summary>
    /// <summary>显示名的英文部分。</summary>
    public string NameEn { get; init; } = "";

    /// <summary>The Chinese half of the one-line summary.</summary>
    /// <summary>单行摘要的中文部分。</summary>
    public string SummaryZh { get; init; } = "";

    /// <summary>The English half of the one-line summary.</summary>
    /// <summary>单行摘要的英文部分。</summary>
    public string SummaryEn { get; init; } = "";

    /// <summary>Whether the face owns a registry of its own.</summary>
    /// <summary>该面是否自持注册机。</summary>
    public bool NeedsRegistry { get; init; }
}
