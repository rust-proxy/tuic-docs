# 配置描述 DSL

配置生成器使用 **Config DSL v1** 描述输入字段和输出配置。它是一套嵌入 JavaScript 的声明式语言，定义写在 ES module 中，由浏览器直接解释，不需要代码生成、前端打包或运行时第三方依赖。

DSL 定义由维护者编写，最终下载的配置仍是 TOML、JSON 或 YAML。生成器没有执行、导入用户提供的 DSL 的入口。表单中的文本始终作为数据处理。

## 示例

下面的完整定义展示了表单默认值、条件校验、配置键映射和敏感字段。示例用于演示 DSL，不是 TUIC 配置。

```javascript
import {
  string, boolean, object, defaults, validateSchema, project, redact,
} from '../assets/config-generator/dsl.mjs';

const input = object({
  name: string({ default: 'demo' }),
  auth: boolean(),
  password: string({
    secret: true,
    when: state => state.auth,
    check: value => value.length ? '' : '请填写密码。',
  }),
});

const output = object({
  service_name: string({ from: 'name' }),
  authentication: object({
    password: string({ from: 'password', secret: true }),
  }, { when: state => state.auth }),
});

const state = defaults(input);
const errors = validateSchema(input, state); // {}
const config = project(output, state);       // { service_name: 'demo' }
const preview = redact(output, config);      // 独立的预览对象
```

开启 `auth` 后，输入密码才能通过校验；输出包含 `authentication.password`，预览中该值替换为 `••••••••`。真实导出使用 `config`，不使用隐藏密码后的对象。

## 类型与结构

每个构造器返回一个可遍历的类型描述节点。`object` 的键是字面字段名，嵌套配置必须写成嵌套 `object`，点号不会自动展开成路径。

| 写法 | 数据类型 | 未声明 `default` 时的表单初值 |
| --- | --- | --- |
| `string(options)` | 字符串，不自动裁剪或转换 | `''` |
| `boolean(options)` | 严格布尔值 | `false` |
| `integer(options)` | JavaScript 安全整数 | `0` |
| `choice(choices, options)` | 指定的字符串枚举 | 第一个枚举值 |
| `object(fields, options)` | 固定字段的对象 | 递归生成所有子字段初值 |
| `list(item, options)` | 同一类型的列表 | `[]` |
| `record(item, options)` | 动态字符串键到同一类型值的映射 | `{}` |

`choice` 可写成 `['info', 'warn']`，也可写成 `[['bbr', 'BBR'], ['newreno', 'New Reno']]`，分别提供实际值和界面标签。枚举至少需要一个选项，值必须无重复，且全部为字符串。

例如，TUIC 的 `users` 用 `record(string({ secret: true }), …)` 描述，端口转发用 `list(object({ … }), …)` 描述，`tls` 和 `backend` 用嵌套 `object` 描述。类型不依赖 TOML / JSON / YAML 中的具体写法。

## 节点选项

| 选项 | 含义 |
| --- | --- |
| `default` | 表单初值；每次 `defaults` 都深拷贝显式初值，不共享用户列表等可变数据 |
| `from: 'field'` | 从当前输入作用域读取一个自有字段；不解释点号路径 |
| `from: (scope, root) => value` | 显式转换输入，如裁剪路径、生成带单位的持续时间、过滤转发列表 |
| `value` | 输出常量，例如 `true` 或 `['h3']`；与 `from` 互斥 |
| `when: (scope, root) => boolean` | 字段是否参与校验或输出；表单也读取该条件控制文本字段显示 |
| `check: (value, scope, root) => message` | 类型通过后的约束；返回空字符串表示通过，非空字符串为错误信息 |
| `secret: true` | 按输出结构递归隐藏该节点的值；不影响真实导出 |
| `ui` | 表单元数据：`label`、`placeholder`、`hint`、`anchor`、可选 `type` 和 `numeric` |

未知选项、重复枚举、非法子节点和同时使用 `from` / `value` 会在定义时抛错。回调是受信任的 JavaScript 函数，应保持纯函数，不修改输入，不访问网络或浏览器存储，不把字段值写入错误信息。

`default` 只供初始化使用。校验和输出不会把缺失值或错误值替换成默认值，也不会把字符串 `'false'` 自动转换为布尔值。表单中的端口与持续时间暂存为字符串，以便展示空白和无效输入；通过校验后才在 `from` 中显式转换。

## 作用域与求值

`project(output, state)` 从根输入 `state` 开始遍历：

1. 在当前输入作用域执行 `when`。条件为假时直接省略整个节点，不执行它的 `from`、`check` 或子节点。
2. 从 `value`、`from` 或当前作用域取得来源。未写 `from` 的对象块沿用作用域，不会按输出键自动切换到同名输入对象。
3. 对象按字段定义构造输出；列表和映射中的每个元素成为其子节点的作用域；`root` 始终是原始根输入。
4. 检查生成值的类型和 `check` 约束。错误通过 `ConfigDslError.errors` 返回字段路径，例如 `client.local.udp_forward.0.timeout`。

因此，普通输出标量通常写成 `string({ from: '字段名' })`；集合中的 `string()` 则直接使用当前元素。对象可用 `{ from: 'settings' }` 切换作用域，再由子字段读取其中的值。

假值 `false`、数字 `0`、空字符串和空列表都是真实值，不代表省略。需要省略字段时必须明确声明 `when`。省略转发空列表、关闭认证后不输出用户名密码、切换证书模式后不输出旧字段，都由这些条件控制。

## 校验、预览与序列化

- `defaults(input)`：构造所有分支的初始表单状态，包括暂时隐藏的字段；不执行条件或映射。
- `validateSchema(input, state)`：检查活动字段的类型和约束，返回以字段路径为键的错误对象。不会应用输出映射，也不会改写输入。
- `validateSchema(input, state, { typesOnly: true })`：检查完整内部状态的类型，包括隐藏字段；不执行条件和业务约束，枚举仅检查字符串类型。生成器先执行这一层，避免关联规则处理畸形数据。
- `project(output, state)`：在输入校验成功后，解释输出定义，生成纯数据对象。只生成声明的字段。
- `redact(output, config)`：遍历已生成的配置并隐藏 `secret` 节点；不重新执行输入映射或条件，不修改原始配置。出现未声明的对象字段时拒绝生成预览。
- `serialize(config, format)`：现有序列化器将纯数据对象转换为 TOML、JSON 或 YAML，并处理字符串转义。

`validateSchema` 检查已声明的输入字段，允许额外输入键，但 `project` 不会把它们自动传入配置。这套 DSL 描述生成器支持的配置子集，不是完整的 TUIC 配置解析器，不提供配置导入或任意语言的执行沙箱。

## 生成器中的分工

| 文件 | 职责 |
| --- | --- |
| `dsl.mjs` | 通用类型、初始化、字段校验、配置映射与密码隐藏 |
| `schema.mjs` | `INPUT`、`USER`、`FORWARD` 输入定义，以及服务端和客户端 `OUTPUT` 定义 |
| `addresses.mjs` | 域名、IP、端口与监听地址的公共解析函数 |
| `validation.mjs` | 调用 DSL 校验，补充配对 SNI、重复 UUID、端口冲突、重连上下限等关联规则 |
| `model.mjs` | 输入校验通过后调用 `project`，提供配置预览的隐藏密码入口 |
| `app.mjs` | 表单布局、动态列表和交互；读取 DSL 的字段元数据、条件、枚举及初值 |
| `serializers.mjs` | 输出格式及转义 |

表单分区、生成凭据、用户选择和 TLS 切换时关闭“跳过证书校验”等交互仍由应用层负责。跨字段的配对和安全规则保留为显式业务逻辑，便于与对应 TUIC 版本核对。

新增字段时，在 `INPUT` 中描述初值、活动条件、字段校验和界面元数据，在 `OUTPUT` 中声明真实配置键、类型及映射，再将控件放入相应表单分区。敏感字段应同时标注输入和输出的 `secret`，并补充对应控件类型。不要直接修改 `model.mjs` 拼装配置或在预览层另列密码字段。

新增字段若改变配置含义，还应更新[生成器说明](config-generator-reference.md)，并运行 README 中的 DSL、模型、格式往返、真实解析器及浏览器测试。DSL 声明本身不证明远端服务可用。
