//! Compile-time construction contracts for registration faces.
//! 注册面的编译期构造合同。
//!
//! A preset declares the parts it expects, a parts type declares what it
//! supplies, and [`assert_contract`] forces the two output types to match during
//! macro expansion. [`ObjectContract`] is the runtime-readable record of that
//! relationship.
//! preset 声明它要求的 parts，parts 类型声明它提供的 parts，[`assert_contract`] 在宏
//! 展开时强制两者的输出类型相同。[`ObjectContract`] 是这层关系的运行时可读记录。

use super::*;

/// A preset declares the construction shape it expects.
/// preset 声明它要求的构造形状。
pub trait PresetContract {
    /// Concrete type this preset constructs.
    /// 该 preset 构造出的具体类型。
    type Output;
    /// Part names that must all be supplied before the preset can construct.
    /// 该 preset 完成构造前必须提供的全部 part 名称。
    const REQUIRED_PARTS: &'static [&'static str];
}

/// Parts declare the construction shape an object supplies.
/// parts 声明 object 实际提供的构造形状。
pub trait PartsContract {
    /// Concrete type these parts construct.
    /// 这些 parts 构造出的具体类型。
    type Output;
    /// Part names this type actually supplies.
    /// 该类型实际提供的 part 名称。
    const PROVIDED_PARTS: &'static [&'static str];
}

/// Marker preset for a face that is constructed without a preset.
/// 不需要 preset 即可构造的注册面所使用的标记类型。
pub struct NoPreset;

impl PresetContract for NoPreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &[];
}

/// Marker parts for a face that supplies no parts.
/// 不提供任何 part 的注册面所使用的标记类型。
pub struct NoParts;

impl PartsContract for NoParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &[];
}

/// The **type-like** half of a face declaration (H/A2e), as associated types.
/// 面声明里**类型类**的那一半（H/A2e），写成关联类型。
///
/// A struct literal cannot carry a type, so the two declarations that *are* types — the preset that
/// constructs the face and the parts type that supplies its construction parts — live here, where they
/// are ordinary Rust: completion works, and a wrong type is a compile error. `Params`/`Handle` are not
/// here on purpose: today those are written as a **value** and an **identifier** in the macro
/// (`params: $params:expr`, `handle: $handle:ident`), so a type-level carrier would be a second way to
/// say the same thing.
/// 结构体字面量装不下类型，因此那两个**本身就是类型**的声明——构造这个面的 preset、提供其构造 parts 的
/// parts 类型——住在这里，而这里是普通 Rust：补全可用、写错类型就是编译错误。`Params`/`Handle` **有意不在**
/// 这里：今天它们在宏里分别写成一个**值**和一个**标识符**（`params: $params:expr`、`handle: $handle:ident`），
/// 给它们再造一个类型级载体，等于同一件事有第二种写法。
pub trait FaceContract {
    /// Preset type that constructs this face.
    /// 构造本面的 preset 类型。
    type Preset: PresetContract;
    /// Parts type that supplies this face's construction parts, tied to the preset's output exactly as
    /// [`assert_contract`] ties them.
    /// 提供本面构造 parts 的 parts 类型；其输出与 preset 的输出的绑定，与 [`assert_contract`] 完全一致。
    type Parts: PartsContract<Output = <Self::Preset as PresetContract>::Output>;

    /// The handle-contract labels this face carries, as the runtime registry reads them.
    /// 本面携带的 handle 合同标签，按运行期注册机读取的样子。
    ///
    /// The declaration macros derive these from the contract **types** they were given, and check with
    /// `__assert_impls!` that the face implements each of them. That check has no macro-free equivalent —
    /// Rust cannot quantify a bound over a variable list of traits — so a hand-written face names the
    /// labels here and keeps the check beside it. The default is "no contracts", which is what a face
    /// that declares none should say rather than an empty list invented by its consumer.
    /// 声明宏从它们拿到的合同**类型**推导这些标签，并用 `__assert_impls!` 检查这个面实现了其中每一个。那条检查
    /// **没有**无需宏的等价物——Rust 无法让一个 bound 对**变长**的 trait 列表做量化——因此手写的面在这里点名
    /// 标签，并把那条检查写在旁边。"没有合同"是默认值，也正是没有声明合同的面**应当**说的，而不是由它的消费者
    /// 编一个空列表出来。
    const HANDLE_TRAITS: &'static [&'static str] = &[];

    /// The part-contract labels this face carries; see [`FaceContract::HANDLE_TRAITS`].
    /// 本面携带的 part 合同标签；见 [`FaceContract::HANDLE_TRAITS`]。
    const PART_TRAITS: &'static [&'static str] = &[];
}

/// The compile-time check a face can make about its own two type-like declarations.
/// 一个面就自己那两个类型类声明所做的编译期检查。
///
/// It exists so the trait has a user the compiler can see — measured: a `pub trait` nothing uses is
/// `dead_code` under `-D warnings`, while a `pub` struct in the same module is not.
/// 它的存在让那个 trait 有一个编译器看得见的使用者——实测：无人使用的 `pub trait` 在 `-D warnings` 下是
/// `dead_code`，而同一模块里的 `pub` struct 不是。
pub const fn assert_face_contract<F: FaceContract>() {}

/// Force preset and parts output types to match during macro expansion.
/// 在宏展开时强制 preset 与 parts 的输出类型相同。
///
/// ```compile_fail
/// use xirang_kernel::{assert_contract, PartsContract, PresetContract};
///
/// struct Expected;
/// struct Supplied;
/// impl PresetContract for Expected {
///     type Output = Expected;
///     const REQUIRED_PARTS: &'static [&'static str] = &[];
/// }
/// impl PartsContract for Supplied {
///     type Output = Supplied;
///     const PROVIDED_PARTS: &'static [&'static str] = &[];
/// }
///
/// const _: () = assert_contract::<Expected, Supplied>();
/// ```
pub const fn assert_contract<P, T>()
where
    P: PresetContract,
    T: PartsContract<Output = P::Output>,
{
}

impl ObjectContract {
    /// Copy this borrowed contract into the owned form a snapshot can retain.
    /// 将该借用合同复制为快照可长期持有的拥有所有权形式。
    pub fn into_owned(self) -> OwnedObjectContract {
        OwnedObjectContract {
            required_parts: self
                .required_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            provided_parts: self
                .provided_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }

    /// Return one message per unmet part requirement; an empty result means the
    /// contract holds. `object` names the face being checked.
    /// 每个未满足的 part 要求各返回一条消息；结果为空表示合同成立。
    /// `object` 是被检查的注册面名称。
    ///
    /// The output half of this check is gone with the two names it compared: a
    /// type-level `assert_contract` proves the same fact at compile time, per
    /// face, and a graft cut proves it across two faces. This method now reports
    /// only what the trait constants say, which is the part no compiler can see.
    /// 本检查的输出那一半随它比较的那两个名字一起删除：同一件事由编译期
    /// `assert_contract` 按面证明、由嫁接切口跨两个面证明。本方法现在只报告 trait 常量
    /// 说了算的东西——那是编译器看不到的部分。
    pub fn validate_object_contract(&self, object: &str) -> Vec<String> {
        validate_object_contract(self.required_parts, self.provided_parts, object)
    }
}

/// Runtime-readable form of the construction contract.
/// 构造合同的运行时可读形式。
#[derive(Clone, Copy, Debug)]
pub struct ObjectContract {
    /// Part names the preset declared it needs.
    /// preset 声明它需要的 part 名称。
    pub required_parts: &'static [&'static str],
    /// Part names the parts type supplied.
    /// parts 类型实际提供的 part 名称。
    pub provided_parts: &'static [&'static str],
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod contract_tests;
