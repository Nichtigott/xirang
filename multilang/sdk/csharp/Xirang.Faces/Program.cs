using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Xirang;

namespace Xirang.Faces;

/// <summary>
/// Writes the <c>xirang.faces</c> records a C# package declares.
/// 写出一个 C# 包声明的 <c>xirang.faces</c> 记录。
/// </summary>
/// <remarks>
/// It parses the package's C# sources with Roslyn; it never loads or runs the package. A declaration
/// is a class with one <c>[Soil(…)]</c> attribute whose arguments are literals (and at most a
/// constant the build generated), so the same source that compiles against the SDK is the source the
/// build reads.
/// 它用 Roslyn 解析包的 C# 源码，从不加载或运行包。一条声明是一个带 <c>[Soil(…)]</c> 特性的类，参数是
/// 字面量（最多再加一个构建生成的常量），因此对着 SDK 编译的源码就是构建读取的源码。
/// </remarks>
internal static class Program
{
    private static int Main(string[] args)
    {
        string? source = null;
        string? root = null;
        string? output = null;
        for (int index = 0; index < args.Length; index++)
        {
            switch (args[index])
            {
                case "-source":
                    if (++index >= args.Length)
                    {
                        return Fail("-source needs a directory");
                    }
                    source = args[index];
                    break;
                case "-root":
                    if (++index >= args.Length)
                    {
                        return Fail("-root needs the package root");
                    }
                    root = args[index];
                    break;
                case "-out":
                    if (++index >= args.Length)
                    {
                        return Fail("-out needs a file");
                    }
                    output = args[index];
                    break;
                default:
                    return Fail($"unknown argument `{args[index]}`");
            }
        }
        if (source is null || root is null || output is null)
        {
            return Fail("usage: xirang-faces -source <dir> -root <package root> -out <file>");
        }
        string rootFull = Path.GetFullPath(root);
        List<string> files = Directory
            .EnumerateFiles(source, "*.cs", SearchOption.AllDirectories)
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToList();
        // Two passes: a declaration may name a constant the build generated into another file, and
        // constants are file-local in the syntax trees.
        // 两遍：声明可能点名构建生成在另一个文件里的常量，而常量在语法树里是逐文件的。
        var constants = new Dictionary<string, string>(StringComparer.Ordinal);
        var parsed = new List<(string Path, SyntaxTree Tree)>();
        foreach (string path in files)
        {
            SyntaxTree tree = CSharpSyntaxTree.ParseText(File.ReadAllText(path), path: path);
            parsed.Add((path, tree));
            CollectConstants(tree, constants);
        }
        var declarations = new List<(string Source, FaceFields Fields)>();
        foreach ((string path, SyntaxTree tree) in parsed)
        {
            string spelling;
            try
            {
                spelling = RelativeSource(rootFull, path);
            }
            catch (GeneratorException error)
            {
                return Fail(error.Message);
            }
            foreach (ClassDeclarationSyntax type in tree.GetRoot().DescendantNodes().OfType<ClassDeclarationSyntax>())
            {
                AttributeSyntax? attribute = type.AttributeLists
                    .SelectMany(list => list.Attributes)
                    .FirstOrDefault(candidate => SimpleName(candidate.Name) == "Soil");
                if (attribute is null)
                {
                    continue;
                }
                FaceFields fields;
                try
                {
                    fields = Fields(attribute, constants);
                }
                catch (GeneratorException error)
                {
                    return Fail($"{spelling}: {error.Message}");
                }
                declarations.Add((spelling, fields));
            }
        }
        if (declarations.Count == 0)
        {
            return Fail($"no [Soil] declarations under {source}; a declared record file with no faces is a mistake");
        }
        declarations.Sort((left, right) =>
        {
            int bySource = string.CompareOrdinal(left.Source, right.Source);
            return bySource != 0 ? bySource : string.CompareOrdinal(left.Fields.Named, right.Fields.Named);
        });
        var records = new List<byte>(declarations.Count * RecordWriter.RecordBytes);
        foreach ((string spelling, FaceFields fields) in declarations)
        {
            try
            {
                records.AddRange(RecordWriter.Encode(fields, spelling));
            }
            catch (ArgumentException error)
            {
                return Fail($"{spelling}: {fields.Named}: {error.Message}");
            }
        }
        string? directory = Path.GetDirectoryName(Path.GetFullPath(output));
        if (directory is not null)
        {
            Directory.CreateDirectory(directory);
        }
        File.WriteAllBytes(output, records.ToArray());
        return 0;
    }

    /// <summary>Records every string constant a file declares, for parent resolution.</summary>
    private static void CollectConstants(SyntaxTree tree, Dictionary<string, string> into)
    {
        foreach (FieldDeclarationSyntax field in tree.GetRoot().DescendantNodes().OfType<FieldDeclarationSyntax>())
        {
            if (!field.Modifiers.Any(SyntaxKind.ConstKeyword))
            {
                continue;
            }
            foreach (VariableDeclaratorSyntax variable in field.Declaration.Variables)
            {
                if (variable.Initializer?.Value is LiteralExpressionSyntax literal
                    && literal.IsKind(SyntaxKind.StringLiteralExpression))
                {
                    into[variable.Identifier.Text] = literal.Token.ValueText;
                }
            }
        }
    }

    /// <summary>Reads one <c>[Soil(…)]</c> attribute into the SDK's field set, refusing unknowns.</summary>
    private static FaceFields Fields(AttributeSyntax attribute, Dictionary<string, string> constants)
    {
        var fields = new FaceFields();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        if (attribute.ArgumentList is null)
        {
            throw new GeneratorException("a `[Soil]` attribute names no fields");
        }
        foreach (AttributeArgumentSyntax argument in attribute.ArgumentList.Arguments)
        {
            if (argument.NameEquals is null)
            {
                throw new GeneratorException("every `[Soil]` argument is `Name = value`");
            }
            string name = argument.NameEquals.Name.Identifier.Text;
            if (!seen.Add(name))
            {
                throw new GeneratorException($"the declaration names `{name}` twice");
            }
            switch (name)
            {
                case "Parent":
                    fields.Parent = StringValue(argument.Expression, constants);
                    break;
                case "Named":
                    fields.Named = StringValue(argument.Expression, constants);
                    break;
                case "RegistryName":
                    fields.RegistryName = StringValue(argument.Expression, constants);
                    break;
                case "StableName":
                    fields.StableName = StringValue(argument.Expression, constants);
                    break;
                case "Exports":
                    fields.Exports = StringArray(argument.Expression, constants, name);
                    break;
                case "Provides":
                    fields.Provides = StringArray(argument.Expression, constants, name);
                    break;
                case "Requires":
                    fields.Requires = StringArray(argument.Expression, constants, name);
                    break;
                case "Preset":
                    fields.Preset = StringValue(argument.Expression, constants);
                    break;
                case "Parts":
                    fields.Parts = StringValue(argument.Expression, constants);
                    break;
                case "NameZh":
                    fields.NameZh = StringValue(argument.Expression, constants);
                    break;
                case "NameEn":
                    fields.NameEn = StringValue(argument.Expression, constants);
                    break;
                case "SummaryZh":
                    fields.SummaryZh = StringValue(argument.Expression, constants);
                    break;
                case "SummaryEn":
                    fields.SummaryEn = StringValue(argument.Expression, constants);
                    break;
                case "NeedsRegistry":
                    fields.NeedsRegistry = Bool(argument.Expression, name);
                    break;
                default:
                    throw new GeneratorException(
                        $"unknown named argument `{name}` in a `[Soil]` declaration; the generator names " +
                        "a typo rather than dropping the field");
            }
        }
        if (fields.Parent.Length == 0)
        {
            throw new GeneratorException("a `[Soil]` declaration must name `Parent`");
        }
        if (fields.Named.Length == 0)
        {
            throw new GeneratorException("a `[Soil]` declaration must name `Named`");
        }
        return fields;
    }

    /// <summary>Reads one string literal, or one constant the build generated.</summary>
    private static string StringValue(ExpressionSyntax expression, Dictionary<string, string> constants)
    {
        switch (expression)
        {
            case LiteralExpressionSyntax literal when literal.IsKind(SyntaxKind.StringLiteralExpression):
                return literal.Token.ValueText;
            case MemberAccessExpressionSyntax member:
                return Constant(constants, member.Name.Identifier.Text);
            case IdentifierNameSyntax identifier:
                return Constant(constants, identifier.Identifier.Text);
            case ParenthesizedExpressionSyntax parenthesized:
                return StringValue(parenthesized.Expression, constants);
        }
        throw new GeneratorException(
            "a `[Soil]` value is not a string literal or a generated constant");
    }

    /// <summary>Reads one generated constant, refusing a name the package does not declare.</summary>
    private static string Constant(Dictionary<string, string> constants, string name)
    {
        if (constants.TryGetValue(name, out string? value))
        {
            return value;
        }
        throw new GeneratorException(
            $"`{name}` is not a constant this package declares; the build writes the generated " +
            "`Faces.cs` with `XirangFaces.RootNodeId`");
    }

    /// <summary>Reads a `new[] { … }` or `new string[] { … }` literal.</summary>
    private static string[] StringArray(
        ExpressionSyntax expression,
        Dictionary<string, string> constants,
        string field)
    {
        InitializerExpressionSyntax? initializer = expression switch
        {
            ImplicitArrayCreationExpressionSyntax implicitArray => implicitArray.Initializer,
            ArrayCreationExpressionSyntax array => array.Initializer,
            _ => null,
        };
        if (initializer is null)
        {
            throw new GeneratorException($"`{field}` is not a `new[] {{ … }}` array literal");
        }
        var values = new List<string>(initializer.Expressions.Count);
        foreach (ExpressionSyntax element in initializer.Expressions)
        {
            values.Add(StringValue(element, constants));
        }
        return values.ToArray();
    }

    /// <summary>Reads `true` or `false`.</summary>
    private static bool Bool(ExpressionSyntax expression, string field)
    {
        if (expression is LiteralExpressionSyntax literal)
        {
            if (literal.IsKind(SyntaxKind.TrueLiteralExpression))
            {
                return true;
            }
            if (literal.IsKind(SyntaxKind.FalseLiteralExpression))
            {
                return false;
            }
        }
        throw new GeneratorException($"`{field}` is not `true` or `false`");
    }

    /// <summary>The package-relative spelling a face's identity is derived from.</summary>
    private static string RelativeSource(string rootFull, string path)
    {
        string spelling = Path.GetRelativePath(rootFull, Path.GetFullPath(path)).Replace('\\', '/');
        if (spelling == ".." || spelling.StartsWith("../", StringComparison.Ordinal))
        {
            throw new GeneratorException(
                $"{Path.GetFullPath(path)} is outside the package root {rootFull}; a face's " +
                "identity must be a package-relative source spelling");
        }
        return spelling;
    }

    /// <summary>The simple name of an attribute name, including `Xirang.Soil` and `SoilAttribute`.</summary>
    private static string SimpleName(NameSyntax name) => name switch
    {
        IdentifierNameSyntax identifier => identifier.Identifier.Text,
        QualifiedNameSyntax qualified => SimpleName(qualified.Right),
        AliasQualifiedNameSyntax alias => alias.Name.Identifier.Text,
        _ => "",
    };

    /// <summary>Reports one generator error and returns a non-zero exit code.</summary>
    private static int Fail(string message)
    {
        Console.Error.WriteLine($"xirang-faces: {message}");
        return 1;
    }

    /// <summary>One refusal the generator reports by name.</summary>
    private sealed class GeneratorException(string message) : Exception(message);
}
