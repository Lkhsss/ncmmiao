use clap::Parser;

#[derive(Parser)]
#[command(name = "ncmmiao")]
#[command(author = "lkhsss")]
#[command(version,about = "一个解密ncm文件的神秘程序 By Lkhsss", long_about = None)]
pub struct Cli {
    /// 并发的最大线程数，默认为8线程
    #[arg(short, long)]
    pub workers: Option<usize>,
    /// 需要解密的文件夹或文件
    #[arg(short, long, name = "输入文件/目录")]
    pub input: Vec<String>,

    /// 输出目录
    #[arg(short, long, name = "输出目录", default_value = "NcmmiaoOutput")]
    pub output: Option<String>,

    /// 强制覆盖保存开关
    #[arg(short, long, name = "强制覆盖开关")]
    pub forcesave: bool,

    /// 自动打开输出目录
    #[arg(short,long,name="自动打开输出目录")]
    pub autoopen:bool,
}
