package xirang;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * One XiRang registration face, declared on a Java type.
 * 一个声明在 Java 类型上的 XiRang 注册面。
 *
 * <p>The annotation is a carrier, not a runtime transformation: it is {@code SOURCE}-retained, so it
 * disappears from the compiled class, and a build-time annotation processor turns it into one
 * {@code xirang.faces} record — the same 1,228-byte version-3 record the C, C++ and Zig SDKs write.
 * 注解是载体而不是运行时变换：它是 {@code SOURCE} 保留的，因此不会进入编译后的 class；构建期注解处理器
 * 把它变成一条 {@code xirang.faces} 记录——与 C、C++、Zig SDK 写的是同一种 1,228 字节版本 3 记录。
 *
 * <p>{@code parent} is the generated constant's value, not a spelling the processor has to resolve:
 * the build generates a {@code xirang.Faces} class whose {@code ROOT_NODE_ID} (and, later, per-face
 * constants) are hex strings, and the declaration writes, for example,
 * {@code @Soil(parent = Faces.ROOT_NODE_ID, named = "Gauge")}. A wrong parent is refused when the
 * tree is assembled, by name.
 * {@code parent} 是生成常量的值，而不是处理器要去解析的拼写：构建生成一个 {@code xirang.Faces} 类，
 * 其中的 {@code ROOT_NODE_ID}（以后还有逐面常量）是十六进制字符串，声明写成例如
 * {@code @Soil(parent = Faces.ROOT_NODE_ID, named = "Gauge")}。父级写错会在组装树时按名拒绝。
 */
@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.TYPE)
public @interface Soil {
    /**
     * The parent identity, as 32 lowercase hex characters (the generated constant's value).
     * 父身份，32 个小写十六进制字符（生成常量的取值）。
     *
     * @return the parent's identity bytes
     */
    String parent();

    /**
     * The display name, which is the kind the registry keys on.
     * 显示名，也就是注册机据以索引的 kind。
     *
     * @return the face name inside its parent
     */
    String named();

    /**
     * The name the registry keys this face by, when it differs from {@link #named()}.
     * 注册机索引本面所用的名字（当它与 {@link #named()} 不同时）。
     *
     * @return the registry key, or an empty string to use the default
     */
    String registry_name() default "";

    /**
     * The author-owned identity that survives a source move.
     * 作者拥有的、可跨源码移动保持的身份名。
     *
     * @return the stable name, or an empty string to derive one
     */
    String stable_name() default "";

    /**
     * Export names.
     * 导出名称。
     *
     * @return the export names
     */
    String[] exports() default {};

    /**
     * Capability names this face provides.
     * 本面提供的能力名。
     *
     * @return the provided capability names
     */
    String[] provides() default {};

    /**
     * Capability requirements, each spelled {@code capability=>provider}.
     * 能力需求，每项拼作 {@code capability=>provider}。
     *
     * @return the requirements
     */
    String[] requires() default {};

    /**
     * The preset type path.
     * preset 类型路径。
     *
     * @return the preset path
     */
    String preset() default "";

    /**
     * The parts type path.
     * parts 类型路径。
     *
     * @return the parts path
     */
    String parts() default "";

    /**
     * Localized display name, Chinese half.
     * 本地化显示名，中文部分。
     *
     * @return the Chinese display name
     */
    String name_zh() default "";

    /**
     * Localized display name, English half.
     * 本地化显示名，英文部分。
     *
     * @return the English display name
     */
    String name_en() default "";

    /**
     * Localized summary, Chinese half.
     * 本地化摘要，中文部分。
     *
     * @return the Chinese summary
     */
    String summary_zh() default "";

    /**
     * Localized summary, English half.
     * 本地化摘要，英文部分。
     *
     * @return the English summary
     */
    String summary_en() default "";

    /**
     * Whether the face owns a registry of its own, i.e. children may hang under it.
     * 该面是否自持注册机，也就是子面是否可以挂在它下面。
     *
     * @return true when the face owns a registry
     */
    boolean needs_registry() default false;
}
