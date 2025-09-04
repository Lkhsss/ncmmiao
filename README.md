# NcmMiao :tada:
[![build](https://github.com/Lkhsss/NcmMiao/actions/workflows/build.yml/badge.svg?event=push)](https://github.com/Lkhsss/NcmMiao/actions/workflows/build.yml)

一个使用Rust语言编写的ncm文件解密工具😆。

### 功能及特点
 - 支持单一文件，多文件夹递归批量解密。
 - 完善的日志功能
 - Colorful
 - 编译文件小，解密快
 - 支持自动添加封面！
 - 自动打开输出文件夹
 - 简约美观

## 编译
```
cargo build -r
```

## 使用
支持单一文件，多文件夹递归批量解密。
```
Usage: ncmmiao [OPTIONS]

Options:
  -w, --workers <WORKERS>  并发的最大线程数，默认为cpu核心数
  -i, --input <输入文件/目录>    需要解密的文件夹或文件
  -o, --output <输出目录>      输出目录 [default: NcmmiaoOutput]
  -f, --forcesave          强制覆盖保存开关
  -a, --autoopen           自动打开输出目录
  -n, --nocolor            是否关闭彩色输出。在不支持真彩色的老机型中关闭。
  -d, --debug...           设定输出日志的的等级。v越多日志越详细
  -h, --help               Print help
  -V, --version            Print version
```
### 例
文件位于`D:/Music`中，要求使用64线程，并且完成后自动打开文件夹，不强制覆盖保存
```bash
ncmmiao -i "D:/Music" -w 64 -a
```

### 关于`覆盖保存`
覆盖保存用于当解密进行一半时突然终止程序，不知道程序是否已解密完成时启用，可以强制覆盖已存在的文件，保证文件完整性。

### 关于日志系统
日志共六个等级: Error Warn Info Debug Trace
v越多，日志越详细。
仅输出错误的日志 Error
```bash
ncmmiao -v
```
输出警告以上的日志 Warn
```bash
ncmmiao -vv
```
输出提示以上的日志 INFO (默认)
```bash
ncmmiao -vvv
```
以此类推


---

# TODO :construction:
 - [x] 多线程支持
 - [x] 自动添加封面
 - [x] 解密进度条
 - [x] 命令行解析
 - [x] 自定义输出文件夹
 - [x] 计时功能
 - [x] 自动覆盖开关
 - [x] 优化并发设置
 - [x] 优化信息传递
 - [x] 颜色控制


---
# [Changelog](CHANGELOG.md)
---

# 附 - ncm文件结构
|信息|大小|作用|
|:-:|:-:|:-:|
|Magic Header|8 bytes|文件头|
|Gap|2 bytes||
|Key Length|4 bytes|RC4密钥长度，字节是按小端排序。|
|Key Data|Key Length|RC4密钥|
|Music Info Length|4 bytes|用AES128加密后的音乐相关信息的长度，小端排序。|
|Music Info Data|Music Info Length|Json格式音乐信息数据。|
|Gap|5 bytes||
|CRC校验码|4 bytes|图片的CRC32校验码，小端排序。|
|Image Size|4 bytes|图片的大小|
|Image Data|Image Size|图片数据|
|Music Data||音乐数据|
---
### Magic Header
### Key Data
用AES128加密后的RC4密钥。
1. 先按字节对0x64进行异或。
2. AES解密,去除填充部分。
3. 去除最前面'neteasecloudmusic'17个字节，得到RC4密钥。
### Music Info Data
Json格式音乐信息数据。
1. 按字节对0x63进行异或。
2. 去除最前面22个字节。
3. Base64进行解码。
4. AES解密。
6. 去除前面6个字节，后面数量为最后一个字节的字节数的垃圾数据，得到Json数据。

### Music Data
1. RC4-KSA生成S盒。
2. 用S盒解密(自定义的解密方法)，不是RC4-PRGA解密。


