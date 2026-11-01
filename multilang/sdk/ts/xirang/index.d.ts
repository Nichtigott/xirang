/**
 * The TypeScript side of a XiRang registration face.
 * XiRang 注册面的 TypeScript 一侧。
 *
 * The runtime half is the JavaScript SDK, re-exported here so a `.ts` module and a `.js` module
 * import the same `xirang` and register into the same collector; this package's `types` entry is
 * what makes `tsc` check the declaration object before the code runs.
 * 运行期那一半就是 JavaScript SDK，在这里重导出，因此 `.ts` 模块与 `.js` 模块导入同一个 `xirang`、
 * 登记进同一个收集器；本包的 `types` 条目正是让 `tsc` 在代码运行之前校验声明对象的东西。
 *
 * A face declares itself with one call at module top level:
 * 一个面在模块顶层用一次调用声明自己：
 *
 *     import { faces, soil } from "xirang";
 *
 *     export class Gauge {
 *         reading(): number {
 *             return 42;
 *         }
 *     }
 *
 *     soil.face({
 *         under: faces.shop.panel.NODE_ID,
 *         named: "Gauge",
 *         exports: ["gauge"],
 *         needs_registry: false,
 *     });
 */

/**
 * One generated object-directory leaf: the logical path the build resolves.
 * 生成对象目录的一个叶子：构建要解析的逻辑路径。
 */
export interface NodeId {
    /** The logical path, segments joined by `::`; `root` for the root. */
    /** 逻辑路径，段之间用 `::`；根是 `root`。 */
    readonly NODE_ID: string;
}

/**
 * A fallback object directory: every property is both a directory and a `NODE_ID` leaf.
 * 兜底对象目录：每个属性既是目录，也是一个 `NODE_ID` 叶子。
 *
 * A build writes the real module with one precise attribute per face path; this permissive shape is
 * what the fallback proxy has before that build, so the same source type-checks either way.
 * 构建会写出真实模块，每个面路径一个精确属性；这个宽容的形状就是兜底代理在构建之前的样子，因此同一份
 * 源码在两种情况下都能通过类型检查。
 */
export type FaceDirectory = { readonly [segment: string]: FaceDirectory } & NodeId;

/**
 * The registration fields a `soil.face(…)` call can carry.
 * 一次 `soil.face(…)` 调用能携带的注册字段。
 *
 * The keys are exactly the ones the adapter's grammar table knows. A key outside this interface is a
 * compile error rather than a silently recorded extra: the build preserves unknown keys, but a typo
 * in a declaration is the one thing the type checker exists to catch.
 * 键恰好是适配器语法表认识的那些。本接口之外的键是编译错误而不是被静默记录的额外项：构建会保留未知键，
 * 但声明里的拼写错误正是类型检查器存在的理由。
 */
export interface FaceDeclaration {
    /** The parent, named through the generated directory. */
    /** 父级，经生成目录点名。 */
    under: unknown;
    /** The display name, which is the kind the registry keys on. */
    /** 显示名，也就是注册机据以索引的 kind。 */
    named: string;
    /** Author-owned identity that survives source moves. */
    /** 作者拥有的、可跨源码移动保持的身份名。 */
    stable_name?: string;
    /** Export names. */
    /** 导出名称。 */
    exports?: readonly string[];
    /** Capability names this face provides. */
    /** 本面提供的能力名。 */
    provides?: readonly string[];
    /** `capability=>provider` requirements. */
    /** `capability=>provider` 形式的能力需求。 */
    requires?: readonly string[];
    /** The preset type path. */
    /** preset 类型路径。 */
    preset?: string;
    /** The parts type path. */
    /** parts 类型路径。 */
    parts?: string;
    /** Localized display name, Chinese half. */
    /** 本地化显示名，中文部分。 */
    name_zh?: string;
    /** Localized display name, English half. */
    /** 本地化显示名，英文部分。 */
    name_en?: string;
    /** Localized summary, Chinese half. */
    /** 本地化摘要，中文部分。 */
    summary_zh?: string;
    /** Localized summary, English half. */
    /** 本地化摘要，英文部分。 */
    summary_en?: string;
    /** The external registry this face is provisioned from. */
    /** 本面从哪个外部注册机取得内容。 */
    from_registry?: string;
    /** Whether the face owns a child registry. */
    /** 该面是否拥有子注册机。 */
    needs_registry?: boolean;
}

/** The declaration namespace a face uses. */
/** 面声明使用的命名空间。 */
export declare const soil: {
    /** Declare one face; the object is returned unchanged. */
    /** 声明一个面；对象原样返回。 */
    face(fields: FaceDeclaration): FaceDeclaration;
};

/** The generated object directory, as a fallback proxy before a build writes the real one. */
/** 生成的对象目录，在构建写出真实模块之前以兜底代理的形式存在。 */
export declare const faces: FaceDirectory;

/** The declarations this process recorded, in import order. */
/** 本进程记录下来的声明，按导入顺序。 */
export declare function declarations(): FaceDeclaration[];

/** The face declared under one name, when the module declared one. */
/** 模块以某个名字声明的面（若声明了）。 */
export declare function declared(name: string): FaceDeclaration | undefined;
