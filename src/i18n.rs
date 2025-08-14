use lazy_static::lazy_static;
use std::env;

lazy_static! {
    // 获取当前语言环境
    static ref LANG: String = {
        // 默认语言为英语
        let default_lang = "en".to_string();
        // 从环境变量 LANG 中获取语言信息
        // 格式通常是 `en_US.UTF-8`，我们只需要 `en` 这部分
        env::var("LANG")
            .ok()
            .and_then(|l| l.split('_').next().map(|s| s.to_string()))
            .unwrap_or(default_lang)
    };
}

/// 初始化国际化设置
///
/// 这个函数会根据检测到的操作系统语言来设置当前的 locale。
/// 它应该在程序启动时尽早被调用。
pub fn setup_i18n() {
    // 调用 `set_locale` 来改变当前的语言环境
    rust_i18n::set_locale(&LANG);
}
