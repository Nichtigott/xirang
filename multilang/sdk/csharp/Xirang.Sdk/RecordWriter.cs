using System.Text;

namespace Xirang;

/// <summary>
/// The fields one registration-face declaration carries, for the record writer.
/// 一条注册面声明携带的字段，供记录写入器使用。
/// </summary>
public sealed class FaceFields
{
    /// <summary>The parent identity in hex.</summary>
    public string Parent = "";
    /// <summary>The face name.</summary>
    public string Named = "";
    /// <summary>The registry key.</summary>
    public string RegistryName = "";
    /// <summary>The stable name.</summary>
    public string StableName = "";
    /// <summary>The export names.</summary>
    public string[] Exports = [];
    /// <summary>The provided capabilities.</summary>
    public string[] Provides = [];
    /// <summary>The requirements.</summary>
    public string[] Requires = [];
    /// <summary>The preset path.</summary>
    public string Preset = "";
    /// <summary>The parts path.</summary>
    public string Parts = "";
    /// <summary>The Chinese name.</summary>
    public string NameZh = "";
    /// <summary>The English name.</summary>
    public string NameEn = "";
    /// <summary>The Chinese summary.</summary>
    public string SummaryZh = "";
    /// <summary>The English summary.</summary>
    public string SummaryEn = "";
    /// <summary>Whether the face owns a registry.</summary>
    public bool NeedsRegistry;
}

/// <summary>
/// Writes one version-3 <c>xirang.faces</c> record.
/// 写一条版本 3 的 <c>xirang.faces</c> 记录。
/// </summary>
/// <remarks>
/// The layout is the one <c>multilang/sdk/c/xirang/soil.h</c> writes as a section, plus the
/// appended <c>source[128]</c> field a build-time record file needs: the file is read long after it
/// was written, and the source spelling is part of the face's identity. All multi-byte integers are
/// little-endian. A field too long for its column is refused by name instead of truncated.
/// 布局与 <c>multilang/sdk/c/xirang/soil.h</c> 写进段里的一致，只多出构建期记录文件需要的、追加在末尾的
/// <c>source[128]</c> 字段：文件在被写出很久之后才被读取，而源码拼写是面身份的一部分。所有多字节整数
/// 小端。超出列宽的字段按名拒绝而不是截断。
/// </remarks>
public static class RecordWriter
{
    /// <summary>One record's length.</summary>
    public const int RecordBytes = 1228;
    /// <summary>The source field's offset.</summary>
    public const int SourceOffset = 1100;
    /// <summary>The source field's width.</summary>
    public const int SourceBytes = 128;
    private const ushort Version = 3;

    /// <summary>
    /// Encode one declaration into a version-3 record.
    /// 把一条声明编码成版本 3 记录。
    /// </summary>
    /// <param name="fields">the declaration's fields</param>
    /// <param name="source">the package-relative source spelling</param>
    /// <returns>the record's bytes</returns>
    public static byte[] Encode(FaceFields fields, string source)
    {
        byte[] record = new byte[RecordBytes];
        Encoding.ASCII.GetBytes("XFRC", record.AsSpan(0, 4));
        WriteU16(record, 4, Version);
        WriteU16(record, 6, fields.NeedsRegistry ? (ushort)1 : (ushort)0);
        WriteU16(record, 8, RecordBytes);
        byte[] parent = DecodeParent(fields.Parent);
        parent.CopyTo(record, 12);
        Text(record, 28, 32, "kind", fields.Named);
        Text(record, 60, 48, "registry name", fields.RegistryName);
        Text(record, 108, 32, "stable name", fields.StableName);
        Text(record, 140, 96, "exports", string.Join(",", fields.Exports));
        Text(record, 236, 96, "provides", string.Join(",", fields.Provides));
        Text(record, 332, 128, "requirements", string.Join(",", fields.Requires));
        Text(record, 460, 64, "preset", fields.Preset);
        Text(record, 524, 64, "parts", fields.Parts);
        Text(record, 716, 64, "Chinese name", fields.NameZh);
        Text(record, 780, 64, "English name", fields.NameEn);
        Text(record, 844, 96, "Chinese summary", fields.SummaryZh);
        Text(record, 940, 96, "English summary", fields.SummaryEn);
        Text(record, SourceOffset, SourceBytes, "source", source);
        return record;
    }

    private static void WriteU16(byte[] record, int offset, int value)
    {
        record[offset] = (byte)(value & 0xff);
        record[offset + 1] = (byte)((value >> 8) & 0xff);
    }

    private static byte[] DecodeParent(string parent)
    {
        if (parent.Length != 32)
        {
            throw new ArgumentException(
                $"`Parent` is `{parent}`, not 32 hex characters; use the generated constant " +
                "(`XirangFaces.RootNodeId`) rather than a spelling", nameof(parent));
        }
        byte[] decoded = new byte[16];
        for (int index = 0; index < 16; index++)
        {
            int high = Hex(parent[index * 2]);
            int low = Hex(parent[index * 2 + 1]);
            if (high < 0 || low < 0)
            {
                throw new ArgumentException($"`Parent` is `{parent}`, which is not hexadecimal", nameof(parent));
            }
            decoded[index] = (byte)((high << 4) | low);
        }
        return decoded;
    }

    private static int Hex(char character) => character switch
    {
        >= '0' and <= '9' => character - '0',
        >= 'a' and <= 'f' => character - 'a' + 10,
        >= 'A' and <= 'F' => character - 'A' + 10,
        _ => -1,
    };

    private static void Text(byte[] record, int offset, int width, string role, string text)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(text);
        if (bytes.Length >= width)
        {
            throw new ArgumentException(
                $"the {role} field is {bytes.Length} bytes and the record's column holds {width - 1}; " +
                "shorten it rather than cut an identity in half", role);
        }
        bytes.CopyTo(record, offset);
    }
}
