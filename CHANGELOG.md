# CHANGELOG

## [3.14.0] - 2026.9.1

### Features :sparkles:

- 解密进度条升级：`Signal::Decrypt(f64)` 按字节上报进度（0.0~1.0），MultiProgress 为每个文件单独建立进度条；多文件并发时通过每文件独立通道 + 转发线程实现进度信号的精确归因
- 库 API 拆分：新增 `Ncmfile::dump`（解密后直接返回音乐字节，不保存文件）；原 `dump` 更名 `dump_to_file`（解密并保存到输出目录）
- 新增便捷读取函数：`get_music_info`（返回原始 JSON 字符串）、`get_pic`（返回封面数据）、`get_music_data`（返回解密音乐并携带进度信号）；三者均自动归位游标、可任意顺序调用
- 信号发送器可选化：`tx` 改为 `Option<Sender<Signal>>`，传 `None` 时静默跳过进度上报
- 新增 `NumParseError` 错误类型

### Fixed :bug:

- 修复 `get_pic`/`get_music_data` 漏跳 meta 后固定 9 字节（4+5），导致封面与音乐数据读取错位的严重 bug
- 修复 `seekread` 在剩余字节恰好等于请求长度时误返回空数据的边界问题
- 修复 `seekread`/`skip` 越过文件末尾时 u64 减法下溢 panic
- 修复 meta 数据不足 22 字节、RC4 密钥不足 17 字节时的越界 panic；`unpad` 增加空数据/非法填充防护
- `get_music_info` 改为返回解析前的原始 JSON 字符串（此前返回重序列化结果，键序被打乱）
- `ProtectFile` 检查提前到读取封面/解密之前，已存在文件不再浪费解密计算

### Refactoring

- :hammer: 文件头（magic）校验提前到 `Ncmfile::new`，改为跨平台实现（不再依赖 Windows 专属 API）
- :hammer: 提取 `read_key`/`read_meta`/`read_cover`/`skip_meta`/`skip_cover`/`decrypt_music`/`parse_meta` 等内部 helper，消除 4 处重复的文件头解析
- :hammer: 底层读取 `seekread` 允许短读（文件截断容错），`skip` 跟随 Linux 行为不约束 seek 范围

## [2.14.0] - 2026.7.8

### Features :sparkles:

- 支持 Ctrl+C 优雅关机，首次按提示等待、再次按强制退出
- M4A 格式支持：解密后输出 warn 提示该文件为 AAC 编码、大多数播放器无法播放

### Refactoring

- :hammer: 拆分 ncmdump.rs，加密、密钥、解密表逻辑独立为 cipher.rs 模块
- :hammer: 将 parse_key、unpad 从 Ncmfile 移至 cipher
- :hammer: aes128_to_slice 签名 blocks: Vec<u8> → &[u8]，避免不必要所有权转移
- :hammer: Ncmfile 移除冗余 position 字段，改用 BufReader.stream_position()
- :hammer: 删除 debug_info.rs（与 dump 逻辑重复）、test.rs（无效测试）
- :hammer: 简化 time.rs，移除从未使用的 compare_last/get_time/push_time
- :hammer: 合并 opendir.rs 中 Linux 和 macOS 的重复代码
- :hammer: 线程池简化：删除无用 _id 和 max_workers 字段，新增 shutdown() 公开方法
- :hammer: 线程池消除 Drop 中的 unwrap/panic，改为 channel 断开时优雅退出
- :hammer: 删除 logger.rs 中无意义的 #[cfg(target_os)] 重复分支

### Chore

- :heavy_minus_sign: 移除未使用依赖：mp4ameta、env_logger、image、serde
- :arrow_up: log 显式启用 std feature

## [2.13.27] - 2026.6.25

### Fixed :bug:

- 修正自动构建的依赖问题

## [2.13.26] - 2026.6.25

### Performance :rocket:

- :fire: 封面嵌入改用 metaflac Cursor 从内存解析 STREAMINFO，消除整文件回读（32%→0.06%），I/O 从 3 次降为 1 次
- :fire: 音乐数据预分配容量，消除 Vec 扩容 realloc 开销
- 三轮火焰图优化总加速比约 4x（68,216 → 15,620 samples）
- 预计算RC4解密查找表，每字节省去3次数组查表+2次加法
- 去掉顺序读取时的多余seek调用，让BufReader缓冲机制真正生效
- 解密循环buffer复用，从每次分配32KB降为单次分配
- 消除RC4密钥处理链路中的3次冗余to_vec拷贝
- BufReader/BufWriter缓冲区从8KB增至64KB，减少read/write系统调用

### Upgrade

- :arrow_up: 升级aes到0.9、cipher到0.5、env_logger/log/serde等兼容依赖到最新
- :heavy_plus_sign: 新增metaflac直接依赖，替代audiotags的封面写入路径

### Fixed :bug:

- 修复skip()未推进BufReader游标，导致文件解析偏移错乱的严重bug

### Refactoring

- :hammer: 删除Message.name字段及其无效clone（每文件约6次）
- :hammer: 删除未使用的Metadata/Key结构体、seekread_from/get_fullfilename/fullfilename等死代码
- :hammer: is_ncm签名从Vec<u8>改为&[u8]，parse_key返回值改为()
- :hammer: 清理pathparse无意义写法，删除未使用的导入和FullFilenameError

## [2.11.23] - 2026.5.23

### Performance :rocket:

- 解密流程改为流式写入，降低内存占用
- 复用BufReader并修正读取游标更新，减少IO开销
- 线程池改用crossbeam-channel，降低锁争用
- 限制消息通道容量，减少大批量任务内存占用
- 进度条按任务完成更新，减少UI刷新开销
- 优化AES输出拼接与元数据解析，减少拷贝

## [2.10.23] - 2025.11.13

### Upgrade :sparkles:

- 升级Rust版本！使用更稳健的2024 Edition!
- 升级部份依赖库

## [2.10.22] - 2025.9.7

### Fixed :bug:

- 当未指定输入文件时使用`-a`参数仍然可以打开输出目录
- 更换终端彩色库以支持Windows默认不启用ANSI转义序列的终端
- 更改颜色控制为设置环境变量

## [2.8.20] - 2025.8.26

### Features :sparkles:

- 自动构建增加upx压缩

## [2.7.20] - 2025.8.12

### Features :sparkles:

- 修改默认线程数为cpu核心数
- 修改多线程通信为crossbeam-channel库，增加通讯性能

### Fixed :bug:

- :arrow_up: 升级依赖
- 修正了trace级别日志输出时显示debug级别的问题

### Refactoring

- :hammer: 优雅处理所有的错误
- :hammer: 将代码分离为单个文件
- :hammer: 优化解密算法，提高解密效率

## [2.7.11] - 2025.3.23

### Fixed

- :arrow_up: 升级依赖
- 修正了解密完成后不会自动退出的bug

### Refactoring

- :hammer: 重构部分代码！

## [2.6.11] - 2025.3.15

### Features :sparkles:

- 更新自动打开文件夹选项，当解密结束后自动调用文件管理器打开输出目录

## [2.5.11] - 2025.3.15

- 修正依赖版本号

### Features :sparkles:

- 重新进度条支持！美观了不少啊

### Refactoring

- :hammer: 重构大量代码！

## [2.5.8] - 2025.1.7

### Features :sparkles:

- 增加进度条支持（虽然很丑）
- 增加覆盖保存开关

### Refactoring

- :hammer: 重构代码！使用mpsc进行线程通讯。

## [2.3.7] - 2024.11.24

### Refactoring

- 优化读取逻辑
- :hammer: 重构代码！大量减少panic!

## [2.2.4] - 2024-11-17

### Features :sparkles:

- :ambulance:完整的多线程支持！可自定线程数！
- :ambulance:自动添加封面，解密文件不再需要musictag!
- :sparkles: 完整的命令行参数支持！
- :sparkles: 计时功能！

### Fixed

- :arrow_up: 升级依赖
- 优化保存文件逻辑，保存时间缩短到毫秒

### Refactoring

- :hammer: 重构代码！

## [2.2.4] - 2024-11-17

### Features :sparkles:

- 完整的自动构建流程！

## [1.1.4] - 2024-7-14

### Features :sparkles:

- 增加多线程支持！
  > ~~目前固定4线程，还没写命令行参数。可以源代码修改线程数~~ 已于2.1.4版本修复~

### Fixed

- 优化代码结构

## [1.1.3] - 2024-2-6

### Fixed :bug:

- 修正了部分音乐名称中含有不合法字符时创建文件失败的问题

## [1.1.2] - 2024-2-5

### Fixed :bug:

- 修正了提取音乐信息时，部分歌曲信息提取失败的问题
- 修正了音乐数据解密失败的问题

## [1.1.1] - 2024-2-1

### Features :sparkles:

- 完成批量解密

### Fixed :bug:

- 修正了提取音乐信息会有数据类型错误导致panic的问题

## [1.0.0] - 2024-1-27

### Features :sparkles:

- 初步完成解密函数。主程序成型
