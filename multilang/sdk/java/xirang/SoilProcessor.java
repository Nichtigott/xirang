package xirang;

import com.sun.source.util.Trees;

import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Set;

import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.RoundEnvironment;
import javax.annotation.processing.SupportedAnnotationTypes;
import javax.annotation.processing.SupportedOptions;
import javax.annotation.processing.SupportedSourceVersion;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.Element;
import javax.lang.model.element.TypeElement;
import javax.tools.Diagnostic;

/**
 * Turns every {@link Soil} declaration into one version-3 {@code xirang.faces} record.
 * 把每条 {@link Soil} 声明变成一条版本 3 的 {@code xirang.faces} 记录。
 *
 * <p>The processor is a build-time carrier: it runs inside {@code javac}, reads the source path from
 * the compiler's own trees (so the spelling that becomes part of the identity is the real one, not a
 * guess from a package name), and appends the records to the file named by
 * {@code -Axirang.faces=<path>}. Everything else the record needs is a literal in the annotation.
 * 处理器是构建期载体：它在 {@code javac} 里运行，从编译器自己的树里读源码路径（因此成为身份一部分的拼写
 * 是真的那一个，而不是从包名猜的），并把记录追加到 {@code -Axirang.faces=<path>} 点名的文件。记录需要的
 * 其余东西都是注解里的字面量。
 *
 * <p>Options: {@code -Axirang.faces=<file>} (required) and {@code -Axirang.packageRoot=<dir>} (the
 * root source paths are relativized against; defaults to the working directory).
 * 选项：{@code -Axirang.faces=<file>}（必需）与 {@code -Axirang.packageRoot=<dir>}（源码路径相对它
 * 取拼写；默认工作目录）。
 */
@SupportedAnnotationTypes("xirang.Soil")
@SupportedSourceVersion(SourceVersion.RELEASE_17)
@SupportedOptions({"xirang.faces", "xirang.packageRoot"})
public final class SoilProcessor extends AbstractProcessor {
    @Override
    public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment round) {
        if (annotations.isEmpty()) {
            return false;
        }
        String destination = processingEnv.getOptions().get("xirang.faces");
        if (destination == null) {
            error("the XiRang processor needs `-Axirang.faces=<file>` so it knows where the records go");
            return true;
        }
        Path base = Path.of(processingEnv.getOptions().getOrDefault("xirang.packageRoot", "."))
                .toAbsolutePath()
                .normalize();
        Trees trees = Trees.instance(processingEnv);
        List<Record> records = new ArrayList<>();
        for (TypeElement annotation : annotations) {
            for (Element element : round.getElementsAnnotatedWith(annotation)) {
                Soil soil = element.getAnnotation(Soil.class);
                String source = sourceSpelling(trees, element, base);
                if (source == null) {
                    continue;
                }
                byte[] bytes;
                try {
                    bytes = RecordWriter.encode(new Declaration(soil), source);
                } catch (IllegalArgumentException refusal) {
                    processingEnv.getMessager().printMessage(
                            Diagnostic.Kind.ERROR,
                            "the XiRang declaration on `" + element + "` cannot be written: "
                                    + refusal.getMessage(),
                            element);
                    continue;
                }
                records.add(new Record(source, soil.named(), bytes));
            }
        }
        if (records.isEmpty()) {
            return true;
        }
        records.sort(Comparator.comparing(Record::source).thenComparing(Record::name));
        try (OutputStream stream = new FileOutputStream(destination, true)) {
            for (Record record : records) {
                RecordWriter.append(stream, record.bytes());
            }
        } catch (IOException failure) {
            error("cannot write `" + destination + "`: " + failure);
        }
        return true;
    }

    /**
     * The package-relative spelling of the source a declaration was written in.
     * 一条声明所在源码的包内相对拼写。
     *
     * <p>The spelling comes from the compiler's own source position: an identity derived from a
     * guessed path is a different face, so a declaration whose source the compiler cannot name is an
     * error rather than a record with an invented path.
     * 拼写来自编译器自己的源码位置：由猜测路径推导出的身份是另一个面，因此编译器叫不出源码的声明是错误，
     * 而不是一条带着编造路径的记录。
     */
    private String sourceSpelling(Trees trees, Element element, Path base) {
        var path = trees.getPath(element);
        if (path == null) {
            processingEnv.getMessager().printMessage(
                    Diagnostic.Kind.ERROR,
                    "the XiRang declaration on `" + element + "` has no source in this compilation; "
                            + "compile it from source, not from a class file",
                    element);
            return null;
        }
        Path absolute = Path.of(path.getCompilationUnit().getSourceFile().toUri())
                .toAbsolutePath()
                .normalize();
        if (!absolute.startsWith(base)) {
            processingEnv.getMessager().printMessage(
                    Diagnostic.Kind.ERROR,
                    "`" + absolute + "` is outside the package root `" + base + "`; a face's identity "
                            + "must be a package-relative source spelling",
                    element);
            return null;
        }
        return base.relativize(absolute).toString().replace('\\', '/');
    }

    /**
     * Report one processor-level error through the compiler.
     * 经编译器报告一条处理器级错误。
     */
    private void error(String message) {
        processingEnv.getMessager().printMessage(Diagnostic.Kind.ERROR, message);
    }

    /** One record waiting to be appended, ordered by source then name. */
    private record Record(String source, String name, byte[] bytes) {}

    /** The {@link Soil} annotation seen through the writer's field interface. */
    private record Declaration(Soil soil) implements RecordWriter.Fields {
        @Override
        public String parent() {
            return soil.parent();
        }

        @Override
        public String named() {
            return soil.named();
        }

        @Override
        public String registryName() {
            return soil.registry_name();
        }

        @Override
        public String stableName() {
            return soil.stable_name();
        }

        @Override
        public String[] exports() {
            return soil.exports();
        }

        @Override
        public String[] provides() {
            return soil.provides();
        }

        @Override
        public String[] requires() {
            return soil.requires();
        }

        @Override
        public String preset() {
            return soil.preset();
        }

        @Override
        public String parts() {
            return soil.parts();
        }

        @Override
        public String nameZh() {
            return soil.name_zh();
        }

        @Override
        public String nameEn() {
            return soil.name_en();
        }

        @Override
        public String summaryZh() {
            return soil.summary_zh();
        }

        @Override
        public String summaryEn() {
            return soil.summary_en();
        }

        @Override
        public boolean needsRegistry() {
            return soil.needs_registry();
        }
    }
}
