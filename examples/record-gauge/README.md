# 构建期记录文件承载的面 / A face carried by a build-time record file

`native/gauge.faces` 是一条版本 3 记录（1228 字节：版本 2 布局 + 源码字段），构建读它并登记面。生产者是
`native/write_faces.py`，一个**替身**：真实生产者是 `examples/java-gauge` 里的 javac 注解处理器。

`native/gauge.faces` is one version-3 record (1,228 bytes: the version-2 layout plus the source
field); the build reads it and registers the face. The producer is `native/write_faces.py`, a
**stand-in**: the real producer is the javac annotation processor under `examples/java-gauge`.

```sh
cargo test --offline          # 在 examples/record-gauge 下
tools/xirang-record-file      # 门禁：生产者可复现 + 构建 + 运行期树 + 解码器/表钉子
```
