# 配置描述 DSL

生成器使用 **Config DSL v3**：在 `config-generator/schema/tuic.xml` 中静态描述输入字段、默认值、选项、条件和输出结构，由 **quick-xml + Serde** 反序列化。它是受限的 XML 变种，不包含 Rust 宏、闭包、JavaScript 或其他可执行代码。独立的 Rust + Leptos 页面读取解析结果生成表单，下载格式仍是 TOML、JSON 和 YAML。

v3 替代此前的 Rust 内嵌 DSL；原有表单初值和 TUIC 输出字段保持兼容，无需迁移已经生成的配置。

## 文件结构

一个完整的最小描述如下。这是 DSL 示例，不是 TUIC 配置：

```xml
<config-dsl version="3" target-version="example">
  <inputs>
    <field name="enabled" type="boolean" default="false"
           section="addresses" label="开启认证"/>
    <field name="password" type="string" default=""
           section="addresses" label="密码" widget="password"
           when="auth" rule="password"/>
  </inputs>
  <conditions>
    <condition name="auth"><truthy from="/enabled"/></condition>
  </conditions>
  <outputs>
    <boolean name="enabled" from="/enabled"/>
    <object name="authentication" when="auth">
      <string name="password" from="/password" secret="true"/>
    </object>
  </outputs>
</config-dsl>
```

`inputs` 和 `outputs` 必须各有一个；`conditions` 和 `values` 是可选区块。`version` 固定为 `3`，`target-version` 是目标配置版本。TUIC 描述中的两个顶层输出对象分别命名为 `server` 和 `client`。

## 输入字段

每个 `field` 必须声明 `name`、`type`、`default`、`label`。默认值是静态属性值：

| 类型 | 默认值与控件 |
| --- | --- |
| `string` | 保留原始字符串，普通文本框 |
| `boolean` | 仅接受 `true` 或 `false`，复选框 |
| `integer` | 非负整数，数字输入；用于内部的用户索引 |
| `enum` | 字符串，必须属于子元素 `option` 声明的选项 |

```xml
<field name="controller" type="enum" default="bbr"
       section="transport" label="服务端拥塞控制" when="server">
  <option value="bbr" label="BBR"/>
  <option value="cubic" label="CUBIC"/>
</field>
```

`string` 可指定 `widget="text|password|number|email"`。端口、超时等可编辑数值使用 `type="string" widget="number"`，保留尚未通过校验的输入，便于展示错误；不会提前截断或静默改成零。

可选属性：`placeholder`、`hint`、`section`、`when` 和 `rule`。现有页面分区是 `addresses`、`tls`、`local`、`transport`；`mode`、`format` 和 `internal` 供专用控件使用。`when` 引用一个命名条件，同时控制字段显示和单字段校验。隐藏字段仍保留在页面状态中。

`rule` 选择有限的内置校验规则：`required`、`host`、`port`、`socket`、`email`、`socks-credential`、`milliseconds`、`uuid`、`password`、`endpoint`、`seconds`。规则的地址解析和字节数检查由 Rust 实现，XML 不能注册代码或调用任意函数。

### 用户与转发集合

```xml
<collection name="forwards" initial-items="0" when="client">
  <field name="protocol" type="enum" default="tcp" label="转发协议">
    <option value="tcp" label="TCP"/>
    <option value="udp" label="UDP"/>
  </field>
  <field name="listen" type="string" default=""
         label="本地监听地址" rule="socket"/>
  <field name="remote" type="string" default=""
         label="远端目标地址" rule="endpoint"/>
  <field name="timeout" type="string" default="60"
         label="会话超时（秒）" widget="number" rule="seconds" when="udp"/>
</collection>
```

集合声明行模板，`initial-items` 指定初始行数（0–1000）。新增转发行也读取相同的 XML 默认值。用户集合初始有一行空凭据，页面启动时通过浏览器 Crypto API 填充随机凭据。

行控件读取集合字段的标签、类型、选项和显示条件。每行另有页面内部使用的稳定 `id`，用于增删和焦点保持；它不会出现在导出的配置中。

## 条件与数据来源

条件是 XML 树，没有表达式字符串：

```xml
<conditions>
  <condition name="client"><not><eq from="/mode" value="server"/></not></condition>
  <condition name="localAuth">
    <all><use ref="client"/><truthy from="/localAuth"/></all>
  </condition>
  <condition name="udp"><eq from="protocol" value="udp"/></condition>
</conditions>
```

| 元素 | 含义 |
| --- | --- |
| `all` / `any` | 至少一个子条件，短路检查全部满足或任一满足 |
| `not` | 恰好一个子条件，取反 |
| `use ref="name"` | 引用命名条件 |
| `eq from="…" value="…"` | 将标量表示与属性中的字面字符串比较 |
| `truthy from="…"` | 只接受布尔值，字符串 `"true"` 不等同于布尔值 |
| `ip from="…"` | 检查字符串是否为 IPv4 或 IPv6 地址 |

后三种条件的 `from` 可替换为 `ref`，以读取命名值。`/host` 这样的绝对路径读取顶层输入；`protocol` 这样的相对路径读取当前行字段。输入是扁平字段和集合，不接受点号表达式、任意深层路径、通配符或代码。

## 命名值与固定转换

`values` 复用静态的数据来源描述：

```xml
<values>
  <value name="host"><source from="/host" transform="trim unbracket"/></value>
  <value name="serverHostname">
    <coalesce>
      <source from="/hostname" transform="trim"/>
      <source ref="host" when="client"/>
    </coalesce>
  </value>
</values>
```

命名值始终在顶层上下文中读取。`source` 必须指定一个 `from` 或 `ref`，可使用 `when`。支持以下有限的值元素：

| 元素 | 行为 |
| --- | --- |
| `source` | 读取字段或命名值 |
| `coalesce` | 依次取首个非空字符串、非空值；`false` 和 `0` 不会被跳过；全部为空时返回空字符串 |
| `endpoint` | 恰好两个值元素：主机字符串和整数端口，自动为 IPv6 添加方括号 |
| `select from="/users" index="/activeUser"` | 按非负整数索引选择一行，再读取唯一子值元素；越界时报错 |

`transform` 是固定操作名序列，只支持 `trim`、`lowercase`、`unbracket`、`integer`。每个操作要求字符串来源；`integer` 将十进制数字串规范化为整数，例如 `"045"` → `45`，拒绝负号、小数、指数和溢出。它不是代码片段。

字符串输出可指定 `unit="s"` 或 `unit="ms"`，为整数来源附加时间单位。密码默认原样保留，不自动裁剪空白。

## 输出结构

| 元素 | 输出 |
| --- | --- |
| `object` | 嵌套对象，每个子输出必须声明唯一的静态 `name` |
| `string` / `boolean` / `integer` | 对应标量；整数输出限定为有符号 64 位范围 |
| `enum options="controller"` | 字符串，并检查引用的顶层输入枚举选项 |
| `list from="/forwards"` | 将每一行映射成唯一子输出元素；可用 `where-field` 与 `equals` 过滤 |
| `list`（无 `from`） | 在当前上下文中生成一个元素，例如 ALPN 的 `h3` |
| `record from="/users" key="uuid"` | 动态键映射；唯一子输出描述每个值，可声明 `key-transform` |

标量必须且只能指定一个 `from`、`ref`、`value`，或一个子值元素。`value` 是常量，其类型由输出标签决定，不会将来源字符串隐式转换为布尔值。

```xml
<record name="users" from="/users" key="uuid" key-transform="trim lowercase">
  <string from="password" secret="true"/>
</record>
<string name="password" secret="true">
  <select from="/users" index="/activeUser"><source from="password"/></select>
</string>
<string name="reconnect_initial_backoff" from="/initialBackoff"
        transform="integer" unit="ms" when="reconnect"/>
```

所有输出支持 `when` 与 `secret="true"`。条件不满足时省略整个节点，不读取其内容；`false`、`0`、空字符串和空集合默认保留。集合需声明 `omit-empty="true"` 才会省略空结果。动态映射键经转换后必须非空且唯一，重复键会报错，不会覆盖已有用户。

`secret` 只影响预览脱敏：遍历已经生成的输出树，替换为 `••••••••`，不再次读取输入或计算条件。复制与下载始终使用原始配置。预览遇到未声明字段或类型不符会拒绝处理。

## 解析、校验与错误

`src/dsl/xml.rs` 使用 quick-xml 事件读取器检查受限 XML 子集、完整文档和资源上限，并记录元素位置；`src/dsl/wire.rs` 通过 Serde 标签枚举和 Visitor 反序列化属性及有序子节点；`src/dsl/parser.rs` 检查标签、属性、类型、引用和依赖；`src/dsl.rs` 根据解析树生成对象及脱敏结果。`schema.rs` 通过 `include_str!` 嵌入 XML，并缓存一次解析结果。描述无效时页面显示错误，不生成配置。

语法支持单/双引号属性、自闭合标签、配对标签、空白和注释。支持五种标准实体与十进制/十六进制字符引用。不支持 BOM、XML 声明、DTD、外部实体、命名空间、处理指令、CDATA 或混合文本；不会读取外部文件和网络。属性内容保留解码后的空白，不执行完整 XML 规范的属性空白规范化；当前使用 quick-xml 0.38.4，升级时须保留此行为（0.42 的属性规范化会改变它）。文档大小上限为 1 MiB，元素深度从根元素的 0 开始，最大为 64，在递归反序列化前检查。

解析阶段拒绝未知标签/属性、重复名称、非法默认值、未声明引用、循环条件/值依赖及多重来源。投影阶段检查来源类型、索引、动态键和输出类型。错误包含描述位置或输出路径，不回显字段值。

输入先经过 `validation::validate`：单字段规则与枚举来自 XML；重复 UUID、SNI 必须为域名、监听冲突、重连上下限、自签名必须明确允许跳过验证等跨字段 TUIC 规则留在 Rust。TLS 切换时关闭“跳过证书校验”属于页面交互规则。

## 维护与扩展

1. 在 `schema/tuic.xml` 增加字段、默认值、控件元数据和固定条件；已有分区的普通控件自动渲染。
2. 在同一文件的 `outputs` 中声明实际配置键及敏感标记。新的顶层普通输入可通过页面状态的扩展字段映射保存，无需添加 Rust DSL 构造器。
3. 新增 TUIC 业务关系、控件类型、集合类型或基础转换能力时，修改对应 Rust 实现并增加测试；不在 XML 中嵌入代码。
4. 更新字段说明，运行 Rust 测试、格式化、Clippy、真实 TUIC 解析、三格式独立解析和浏览器检查，命令见仓库 README。

DSL 最终生成 `serde_json::Value`，再交给 TOML、JSON 和 YAML 序列化器。字符串和动态键统一转义，JSON 额外转义 Unicode 行分隔符以兼容 TUIC 的 JSON5 读取器。本 DSL 描述生成器支持的配置子集，不提供旧配置导入；版本基线见[生成器说明](config-generator-reference.md)。
