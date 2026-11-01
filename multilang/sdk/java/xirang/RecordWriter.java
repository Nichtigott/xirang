package xirang;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

/**
 * Writes one version-3 {@code xirang.faces} record.
 * 写一条版本 3 的 {@code xirang.faces} 记录。
 *
 * <p>The layout is the same one {@code multilang/sdk/c/xirang/soil.h} and {@code soil.hpp} write as
 * a section, plus the appended {@code source[128]} field that a build-time record file needs: the
 * file is written long before the build reads it, and the source spelling is part of the face's
 * identity, so it cannot be guessed from a path later.
 * 布局与 {@code multilang/sdk/c/xirang/soil.h}、{@code soil.hpp} 写进段里的那份相同，只多出构建期记录
 * 文件需要的、追加在末尾的 {@code source[128]} 字段：文件在构建读取之前很久就写出，而源码拼写是面身份的一部分，
 * 因此不能以后从路径去猜。
 *
 * <p>All multi-byte integers are little-endian. A field too long for its column is refused by name
 * instead of truncated, because a truncated identity is a different face.
 * 所有多字节整数小端。超出列宽的字段按名拒绝而不是截断，因为被截断的身份是另一个面。
 */
public final class RecordWriter {
    private static final byte[] MAGIC = {'X', 'F', 'R', 'C'};
    private static final int VERSION = 3;
    private static final int RECORD_BYTES = 1228;
    private static final int SOURCE_OFFSET = 1100;
    private static final int SOURCE_BYTES = 128;

    private RecordWriter() {}

    /**
     * Encode one face declaration into a version-3 record.
     * 把一条面声明编码成版本 3 记录。
     *
     * @param parent 32 lowercase hex characters naming the parent identity
     * @param kind the face name; the registry keys on it
     * @param registryName the registry key, or an empty string to use the default
     * @param stableName the author-owned stable name, or an empty string
     * @param exports export names
     * @param provides provided capability names
     * @param requires {@code capability=>provider} requirements
     * @param preset the preset type path, or an empty string
     * @param parts the parts type path, or an empty string
     * @param nameZh the Chinese display name, or an empty string
     * @param nameEn the English display name, or an empty string
     * @param summaryZh the Chinese summary, or an empty string
     * @param summaryEn the English summary, or an empty string
     * @param needsRegistry whether the face owns a registry
     * @param source the package-relative source spelling
     * @return the record's bytes
     */
    public static byte[] encode(
            String parent,
            String kind,
            String registryName,
            String stableName,
            String[] exports,
            String[] provides,
            String[] requires,
            String preset,
            String parts,
            String nameZh,
            String nameEn,
            String summaryZh,
            String summaryEn,
            boolean needsRegistry,
            String source) {
        byte[] record = new byte[RECORD_BYTES];
        System.arraycopy(MAGIC, 0, record, 0, MAGIC.length);
        putU16(record, 4, VERSION);
        putU16(record, 6, needsRegistry ? 1 : 0);
        putU16(record, 8, RECORD_BYTES);
        putParent(record, parent);
        putText(record, 28, 32, "kind", kind);
        putText(record, 60, 48, "registry name", registryName);
        putText(record, 108, 32, "stable name", stableName);
        putList(record, 140, 96, "exports", exports);
        putList(record, 236, 96, "provides", provides);
        putList(record, 332, 128, "requirements", requires);
        putText(record, 460, 64, "preset", preset);
        putText(record, 524, 64, "parts", parts);
        putText(record, 716, 64, "Chinese name", nameZh);
        putText(record, 780, 64, "English name", nameEn);
        putText(record, 844, 96, "Chinese summary", summaryZh);
        putText(record, 940, 96, "English summary", summaryEn);
        putText(record, SOURCE_OFFSET, SOURCE_BYTES, "source", source);
        return record;
    }

    /**
     * Append one record to a stream.
     * 把一条记录追加到流。
     *
     * @param stream the destination stream
     * @param record the record's bytes
     * @throws IOException when the stream refuses the write
     */
    public static void append(OutputStream stream, byte[] record) throws IOException {
        stream.write(record);
    }

    /**
     * Encode a whole declaration into one byte array, for callers that keep records in memory.
     * 把一条完整声明编码成一个字节数组，供在内存里保留记录的调用方使用。
     *
     * @param declaration the fields to encode
     * @param source the package-relative source spelling
     * @return the record's bytes
     */
    public static byte[] encode(Fields declaration, String source) {
        return encode(
                declaration.parent(),
                declaration.named(),
                declaration.registryName(),
                declaration.stableName(),
                declaration.exports(),
                declaration.provides(),
                declaration.requires(),
                declaration.preset(),
                declaration.parts(),
                declaration.nameZh(),
                declaration.nameEn(),
                declaration.summaryZh(),
                declaration.summaryEn(),
                declaration.needsRegistry(),
                source);
    }

    /**
     * The field set one record carries; the processor builds one per annotated type.
     * 一条记录携带的字段集；处理器为每个被注解的类型构建一份。
     */
    public interface Fields {
        /** @return the parent identity in hex */
        String parent();
        /** @return the face name */
        String named();
        /** @return the registry key */
        String registryName();
        /** @return the stable name */
        String stableName();
        /** @return the export names */
        String[] exports();
        /** @return the provided capabilities */
        String[] provides();
        /** @return the requirements */
        String[] requires();
        /** @return the preset path */
        String preset();
        /** @return the parts path */
        String parts();
        /** @return the Chinese name */
        String nameZh();
        /** @return the English name */
        String nameEn();
        /** @return the Chinese summary */
        String summaryZh();
        /** @return the English summary */
        String summaryEn();
        /** @return whether the face owns a registry */
        boolean needsRegistry();
    }

    private static void putU16(byte[] record, int offset, int value) {
        record[offset] = (byte) (value & 0xff);
        record[offset + 1] = (byte) ((value >>> 8) & 0xff);
    }

    private static void putParent(byte[] record, String parent) {
        if (parent == null || parent.length() != 32) {
            throw new IllegalArgumentException(
                    "`parent` is `" + parent + "`, not 32 hex characters; use the generated constant "
                            + "(`Faces.ROOT_NODE_ID`) rather than a spelling");
        }
        for (int index = 0; index < 16; index++) {
            int high = Character.digit(parent.charAt(index * 2), 16);
            int low = Character.digit(parent.charAt(index * 2 + 1), 16);
            if (high < 0 || low < 0) {
                throw new IllegalArgumentException(
                        "`parent` is `" + parent + "`, which is not hexadecimal");
            }
            record[12 + index] = (byte) ((high << 4) | low);
        }
    }

    private static void putText(byte[] record, int offset, int width, String role, String text) {
        byte[] bytes = (text == null ? "" : text).getBytes(StandardCharsets.UTF_8);
        if (bytes.length >= width) {
            throw new IllegalArgumentException(
                    "the " + role + " field is " + bytes.length + " bytes and the record's column holds "
                            + (width - 1) + "; shorten it rather than cut an identity in half");
        }
        System.arraycopy(bytes, 0, record, offset, bytes.length);
    }

    private static void putList(byte[] record, int offset, int width, String role, String[] entries) {
        ByteArrayOutputStream joined = new ByteArrayOutputStream();
        if (entries != null) {
            for (int index = 0; index < entries.length; index++) {
                if (index > 0) {
                    joined.write(',');
                }
                byte[] bytes = entries[index].getBytes(StandardCharsets.UTF_8);
                joined.write(bytes, 0, bytes.length);
            }
        }
        putText(record, offset, width, role, joined.toString(StandardCharsets.UTF_8));
    }
}
