# CHANGELOG
## [1.0.0] - 2024-1-27
### Features :sparkles:
- 初步完成解密函数。主程序成型

## [1.1.1] - 2024-2-1
### Features :sparkles:
- 完成批量解密
### Fixed :bug:
- 修正了提取音乐信息会有数据类型错误导致panic的问题

## [1.1.2] - 2024-2-5
### Fixed :bug:
 - 修正了提取音乐信息时，部分歌曲信息提取失败的问题
 - 修正了音乐数据解密失败的问题
## [1.1.3] - 2024-2-6
### Fixed :bug:
 - 修正了部分音乐名称中含有不合法字符时创建文件失败的问题
## [1.1.4] - 2024-7-14
### Features :sparkles:
- 增加多线程支持！
  > ~~目前固定4线程，还没写命令行参数。可以源代码修改线程数~~ 已于2.1.4版本修复~
### Fixed
- 优化代码结构

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

## [2.3.7] - 2024.11.24
### Refactoring
- 优化读取逻辑
- :hammer: 重构代码！大量减少panic!

## [2.5.8] - 2025.1.7
### Features :sparkles:
- 增加进度条支持（虽然很丑）
- 增加覆盖保存开关
### Refactoring
- :hammer: 重构代码！使用mpsc进行线程通讯。


## [2.5.10] - 2025.3.15
### Features :sparkles:
- 重新进度条支持！美观了不少啊
### Refactoring
- :hammer: 重构大量代码！
