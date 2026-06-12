# VirtualBrowser 优势迁移至 DonutBrowser 设计文档

**日期**: 2026-06-11
**状态**: 草案

## 1. 背景与目标

VirtualBrowser 是一款基于定制 Chromium 的指纹浏览器，在指纹控制的粒度和操作便利性方面具有显著优势。本设计的目标是将 VirtualBrowser 的全部优势特性迁移到 DonutBrowser，包括：

- **指纹深度**：AudioContext、ClientRects、Speech Voices、sec-ch-ua、SSL 版本控制等细粒度指纹项
- **操作体验**：批量创建、配置导入导出、IP 自动匹配（时区/语言/地理位置）

### 约束

- **技术路径**：运行时注入（启动参数、环境变量、Firefox prefs、JS 注入），不维护定制浏览器源码
- **目标引擎**：Camoufox (Firefox) + Wayfern (Chromium) 两个引擎均实现
- **依赖策略**：不引入新的外部依赖，使用现有 Rust 生态和工具链
- **优先级**：先指纹深度，后操作体验

### 不支持项（运行时注入路径限制）

以下三项在运行时注入下无法实现，在 UI 中标记为不支持：

- **MAC 地址**：浏览器不向 JS 暴露真实 MAC
- **设备名**：Firefox/Chromium 不通过标准 API 暴露计算机名
- **端口扫描保护**：浏览器层面无法控制出站端口

## 2. 架构

### 方案选择：统一指纹层（方案 A）

在 `src-tauri/src/` 下新建 `fingerprint/` 模块，定义统一的 `FingerprintAdapter` trait，Camoufox 和 Wayfern 各自实现引擎适配器。前端共享同一套指纹配置 UI。

```
src-tauri/src/
├── fingerprint/                 # 新增：统一指纹层
│   ├── mod.rs                   # 公共类型 + FingerprintAdapter trait
│   ├── profile.rs               # FingerprintProfile 数据结构
│   ├── noise.rs                 # Canvas/AudioContext/ClientRects 噪声生成器
│   ├── network.rs               # SSL 版本控制、端口扫描保护
│   ├── identity.rs              # 设备名、MAC、UA、sec-ch-ua
│   ├── generator.rs             # 随机指纹生成（基于现有 Bayesian 网络）
│   ├── camoufox_adapter.rs      # Camoufox 引擎适配（env vars / prefs）
│   └── wayfern_adapter.rs       # Wayfern 引擎适配（launch args / JS 注入）
├── camoufox/config.rs           # 现有，调用 fingerprint 模块
├── wayfern_manager.rs           # 现有，调用 fingerprint 模块
├── profile/types.rs             # 扩展 BrowserProfile.fingerprint
```

### 核心 trait

```rust
/// 引擎无关的启动配置容器。每个引擎的 adapter 写入自己需要的字段。
pub struct EngineLaunchConfig {
    /// Chromium 启动参数（Wayfern 使用）
    pub args: Vec<String>,
    /// 环境变量（Camoufox 使用）
    pub env_vars: HashMap<String, String>,
    /// Firefox prefs（Camoufox 使用）
    pub firefox_prefs: HashMap<String, serde_json::Value>,
    /// 页面加载前注入的 JS 脚本（Wayfern 通过 CDP 注入）
    pub inject_scripts: Vec<String>,
}

pub trait FingerprintAdapter {
    fn apply(
        &self,
        profile: &FingerprintProfile,
        engine_config: &mut EngineLaunchConfig,
    ) -> Result<(), FingerprintError>;
}
```

- **CamoufoxAdapter**：写入 `env_vars` + `firefox_prefs`
- **WayfernAdapter**：写入 `args` + `inject_scripts`

### 数据流

```
前端 UI → Tauri Command → profile.fingerprint (JSON 存储)
                              ↓
                    launch 时读取 fingerprint
                              ↓
              ┌───────────────┼───────────────┐
              ↓                               ↓
     CamoufoxAdapter                 WayfernAdapter
     (env vars + prefs)              (args + JS injection)
              ↓                               ↓
     Camoufox 进程启动               Wayfern 进程启动
```

## 3. 数据模型

### FingerprintProfile

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FingerprintProfile {
    // ── 基础标识 ──
    pub os: Option<String>,
    pub user_agent: Option<UaConfig>,
    pub sec_ch_ua: Option<SecChUaConfig>,
    pub platform: Option<String>,
    pub language: Option<LanguageConfig>,

    // ── 网络 ──
    pub timezone: Option<TimezoneConfig>,
    pub webrtc: Option<WebRtcConfig>,
    pub ssl: Option<SslConfig>,
    pub port_scan: Option<PortScanConfig>,
    pub mac: Option<MacConfig>,

    // ── 地理位置 ──
    pub geolocation: Option<GeoConfig>,

    // ── 屏幕与显示 ──
    pub screen: Option<ScreenConfig>,
    pub fonts: Option<FontConfig>,

    // ── 渲染指纹 ──
    pub canvas: Option<NoiseMode>,
    pub webgl_img: Option<NoiseMode>,
    pub webgl: Option<WebGlConfig>,
    pub audio_context: Option<NoiseMode>,
    pub client_rects: Option<NoiseMode>,
    pub speech_voices: Option<NoiseMode>,

    // ── 硬件 ──
    pub cpu: Option<u32>,
    pub memory: Option<u32>,
    pub device_name: Option<String>,
    pub gpu_acceleration: Option<bool>,

    // ── 隐私与其他 ──
    pub dnt: Option<bool>,
    pub homepage: Option<String>,
    pub cookie_json: Option<String>,
}
```

### 子配置类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseMode {
    Default,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoOrManual {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaConfig {
    pub mode: AutoOrManual,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecChUaConfig {
    pub mode: AutoOrManual,
    pub brands: Vec<SecChUaBrand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecChUaBrand {
    pub brand: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub mode: AutoOrManual,
    pub language: Option<String>,
    pub languages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimezoneConfig {
    pub mode: AutoOrManual,
    pub name: Option<String>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebRtcMode {
    Replace,
    Allow,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcConfig {
    pub mode: WebRtcMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub enabled: bool,
    pub disabled_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanConfig {
    pub enabled: bool,
    pub allowed_ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacConfig {
    pub mode: AutoOrManual,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoConfig {
    pub mode: AutoOrManual,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub precision: Option<u32>,
    pub permission: GeoPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoPermission {
    Ask,
    Allow,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub mode: AutoOrManual,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub mode: FontMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontMode {
    SystemDefault,
    RandomMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebGlConfig {
    pub mode: AutoOrManual,
    pub vendor: Option<String>,
    pub renderer: Option<String>,
}
```

### 与 BrowserProfile 的关系

```rust
pub struct BrowserProfile {
    // ... 现有字段不变 ...
    #[serde(default)]
    pub fingerprint: Option<FingerprintProfile>,
}
```

- `fingerprint = None`：使用引擎默认行为（现有 Bayesian 生成 / Wayfern 默认配置）
- `fingerprint = Some(...)`：使用用户指定的指纹配置

## 4. 运行时注入机制

### 4.1 Camoufox (Firefox) 引擎

| 指纹项 | 注入方式 | 实现说明 |
|--------|---------|---------|
| UA | env var `navigator.userAgent` | 复用现有 browserforge 映射 |
| sec-ch-ua | Firefox pref `network.http.sec-ch-ua.*` | 设置 Sec-CH-UA 相关 headers |
| 语言 | env var `navigator.language` | 复用现有映射 |
| 时区 | env var `timezone.zone` | 复用现有映射 |
| WebRTC | Firefox pref `media.peerconnection.enabled` | 已有 block_webrtc 基础 |
| 地理位置 | env var `geo:longitude/latitude` | 复用现有 geolocation 模块 |
| 分辨率 | env var `screen.width/height` | 复用现有映射 |
| 字体 | `fonts` config key | 复用现有 fonts 模块 |
| Canvas | env var `canvas:aaOffset` | 已实现 |
| WebGL 绘制 | `webgl::sample_webgl()` | 已实现，扩展 vendor/renderer 自定义 |
| WebGL 元数据 | env var `webgl.vendor` / `webgl.renderer` | 扩展现有 webgl 模块 |
| AudioContext | env var `audio:noise_seed` | 新增，生成随机噪声种子 |
| ClientRects | env var `rects:seed` | 新增，DOM rect spoof 种子 |
| Speech Voices | Firefox pref `media.webspeech.synth.dont_voices_list` | 通过 prefs 控制语音列表 |
| CPU | env var `navigator.hardwareConcurrency` | 复用现有映射 |
| Memory | env var `navigator.deviceMemory` | 复用现有映射 |
| SSL | Firefox pref `security.tls.version.min/max` + `security.ssl3.*` | 控制 TLS 版本和密码套件 |
| DNT | Firefox pref `privacy.donottrackheader.enabled` | Firefox 原生支持 |
| GPU 加速 | Firefox pref `webgl.disabled` + `layers.acceleration.disabled` | 已有 block_webgl 基础 |
| MAC/设备名/端口扫描 | 不支持 | 运行时无法实现，UI 标记为不可用 |

### 4.2 Wayfern (Chromium) 引擎

| 指纹项 | 注入方式 | 实现说明 |
|--------|---------|---------|
| UA | `--user-agent="..."` | Chromium 原生启动参数 |
| sec-ch-ua | JS 注入 `Object.defineProperty` | 修改 header 属性 |
| 语言 | `--lang` | Chromium 原生启动参数 |
| 时区 | `--timezone` | Chromium 原生启动参数 |
| WebRTC | JS 注入拦截 `RTCPeerConnection` | 替换/阻止 IP 泄露 |
| 地理位置 | JS 注入 `navigator.geolocation` | 覆盖 `getCurrentPosition` 返回值 |
| 分辨率 | `--window-size` + JS 注入 `screen` | 覆盖 screen 对象属性 |
| 字体 | JS 注入拦截 `document.fonts` | 限制暴露的字体列表 |
| Canvas | JS 注入覆盖 `toDataURL` / `toBlob` | 添加噪声 |
| WebGL | JS 注入覆盖 `getParameter` | 修改 vendor/renderer 返回值 |
| AudioContext | JS 注入覆盖 `getChannelData` / `getFloatFrequencyData` | 添加噪声 |
| ClientRects | JS 注入覆盖 `getBoundingClientRect` | 添加微小偏移 |
| Speech Voices | JS 注入覆盖 `speechSynthesis.getVoices` | 过滤/修改语音列表 |
| CPU | JS 注入覆盖 `navigator.hardwareConcurrency` | 修改属性值 |
| Memory | JS 注入覆盖 `navigator.deviceMemory` | 修改属性值 |
| SSL | `--ssl-version-min/max` | Chromium 原生启动参数 |
| DNT | `--enable-do-not-track` | Chromium 原生启动参数 |
| GPU 加速 | `--disable-gpu` | Chromium 原生启动参数 |
| MAC/设备名/端口扫描 | 不支持 | 运行时无法实现，UI 标记为不可用 |

### 4.3 JS 注入策略（Wayfern）

Wayfern 启动时带 `--remote-debugging-port` 参数开放 CDP 端口。注入流程分两步：

1. **启动浏览器**：传入 `args` 中的启动参数（`--user-agent`、`--lang`、`--timezone` 等原生参数）
2. **连接 CDP 并注入**：浏览器启动后，通过 CDP 连接调用 `Page.addScriptToEvaluateOnNewDocument`，将 `inject_scripts` 中的所有 JS 脚本注册。这些脚本会在后续每个页面（包括首页）的 DOM 构建前执行，覆盖原生 API 返回值

注入脚本由 `wayfern_adapter.rs` 根据 FingerprintProfile 动态生成，仅包含用户启用的修改项。DonutBrowser 已在 `browser_runner.rs` 中管理 CDP 连接，复用现有基础设施。

### 4.4 IP 自动匹配

```rust
pub async fn resolve_auto_fields(
    profile: &mut FingerprintProfile,
    proxy_url: Option<&str>,
) -> Result<(), GeoError> {
    let ip = geolocation::fetch_public_ip(proxy_url).await?;
    let geo = geolocation::get_geolocation(&ip)?;

    if let Some(ref mut lang) = profile.language {
        if matches!(lang.mode, AutoOrManual::Auto) {
            lang.language = Some(geo.locale.as_string());
            lang.languages = Some(vec![geo.locale.as_string()]);
        }
    }
    if let Some(ref mut tz) = profile.timezone {
        if matches!(tz.mode, AutoOrManual::Auto) {
            tz.name = Some(geo.timezone.clone());
        }
    }
    if let Some(ref mut geo_cfg) = profile.geolocation {
        if matches!(geo_cfg.mode, AutoOrManual::Auto) {
            geo_cfg.longitude = Some(geo.longitude);
            geo_cfg.latitude = Some(geo.latitude);
        }
    }
    Ok(())
}
```

复用现有的 `geolocation::fetch_public_ip()` + `geolocation::get_geolocation()` 链路。

## 5. 前端 UI

### 5.1 指纹配置 Tab

在现有 profile 创建/编辑对话框中新增"指纹配置"tab 页，分区展示：

- **用户标识**：UA（默认/自定义 + 随机刷新）、sec-ch-ua（默认/自定义 + 品牌列表编辑）、语言（按 IP 自动开关 + 下拉选择）、时区（按 IP 自动开关 + 下拉选择）
- **网络与隐私**：WebRTC（替换/允许/阻止）、地理位置（权限 + 按 IP 自动 + 经纬度精度）、SSL（启用/禁用 + 版本勾选）、DNT（开关）
- **渲染指纹**：Canvas / WebGL 绘制 / WebGL 元数据 / AudioContext / ClientRects / Speech Voices（各为默认/随机/自定义）
- **硬件**：CPU 核数、内存、分辨率、字体、GPU 加速
- **其他**：首页 URL、Cookie JSON 导入

### 5.2 批量创建

profile 列表页新增"批量创建"按钮，对话框包含：
- 数量输入
- 代理类型选择（默认/不使用/HTTP/HTTPS/SOCKS5）
- 代理 API 链接输入
- 指纹模式（随机/统一模板）

后端调用 `batch_runner` 模块批量创建 profile。

### 5.3 配置导入/导出

- **导出**：选中 profile → 导出 JSON 文件
- **导入**：上传 JSON → 解析并创建 profile（UUID 重新生成）

JSON 格式：
```json
{
  "version": 1,
  "profiles": [
    {
      "name": "Profile 1",
      "browser": "camoufox",
      "fingerprint": { },
      "proxy": { }
    }
  ]
}
```

### 5.4 IP 查询 API 设置

在 Settings 对话框中新增 IP 查询 API 配置项：
- 自定义 API URL
- API Key 配置
- 用于"按 IP 自动匹配"功能的后端数据源

### 5.5 国际化

- 新增 `fingerprint.*` 命名空间
- 更新全部 9 个 locale 文件（en, es, fr, ja, ko, pt, ru, vi, zh）
- 复用 `common.buttons.*` 等已有 key

## 6. 实施阶段

### 阶段一：指纹深度（优先）

1. 创建 `fingerprint/` 模块，定义 `FingerprintProfile` 和 `FingerprintAdapter` trait
2. 实现 `noise.rs`（AudioContext / ClientRects / Speech Voices 噪声生成器）
3. 实现 `network.rs`（SSL 版本控制）
4. 实现 `identity.rs`（sec-ch-ua 品牌管理）
5. 实现 `camoufox_adapter.rs`（映射到 env vars + prefs）
6. 实现 `wayfern_adapter.rs`（映射到 launch args + JS 注入）
7. 扩展 `profile/types.rs` 添加 `fingerprint` 字段
8. 前端新增指纹配置 tab 页
9. IP 自动匹配（resolve_auto_fields）

### 阶段二：操作体验

10. 批量创建功能（前端对话框 + 后端批量逻辑）
11. 配置导入/导出（JSON 格式定义 + 前后端实现）
12. IP 查询 API 设置（Settings UI + 后端配置存储）
13. 全部 9 个 locale 文件的翻译更新

## 7. 测试策略

- **单元测试**：噪声生成器、配置映射、IP 自动匹配逻辑
- **集成测试**：完整 FingerprintProfile → 引擎配置转换链路
- **手动验证**：使用 BrowserLeaks 和 FingerprintJS 验证各指纹项的注入效果
- **回归测试**：确保 `fingerprint = None` 时现有行为不受影响
