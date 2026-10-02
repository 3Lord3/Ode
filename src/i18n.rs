/// UI language. Always taken from the system locale, there is no manual choice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

/// All user-facing strings go through `Tr`. The locale is fixed at startup from
/// the system locale
pub struct Tr(Lang);

impl Tr {
    pub fn new(lang: Lang) -> Self {
        Self(lang)
    }

    pub fn strings(&self) -> &'static Strings {
        match self.0 {
            Lang::Ru => &RU,
            Lang::En => &EN,
        }
    }
}

impl Tr {
    /// Language from the system locale (first entry of glib::get_language_names)
    pub fn from_system() -> Self {
        let lang = crate::i18n::system_lang();
        Tr::new(lang)
    }
}

/// Picks the UI language from the first two letters of the system locale
fn system_lang() -> Lang {
    let names = gtk4::glib::language_names();
    let first = names.first().map(|s| s.as_str()).unwrap_or("");
    if first.starts_with("ru") {
        Lang::Ru
    } else {
        Lang::En
    }
}

pub struct Strings {
    lang: Lang,
    pub app_title: &'static str,
    pub search_placeholder: &'static str,
    pub empty_title: &'static str,
    pub empty_desc: &'static str,
    pub error_title: &'static str,
    pub no_lyrics_title: &'static str,
    pub no_lyrics_desc: &'static str,
    pub open_genius: &'static str,
    pub copy_lyrics: &'static str,
    pub copied: &'static str,
    pub init_search: &'static str,
    pub search_hint: &'static str,
    pub settings: &'static str,
    pub about: &'static str,
    pub clear_cache: &'static str,
    pub cache_desc: &'static str,
    pub cache_cleared: &'static str,
    pub back: &'static str,
    pub lyrics_attribution: &'static str,
    pub search_results: &'static str,
    pub select_title: &'static str,
    pub select_desc: &'static str,
    pub annot_title: &'static str,
    pub general_tab: &'static str,
    pub no_token_title: &'static str,
    pub no_token_desc: &'static str,
    pub close: &'static str,
    pub no_lyrics_action: &'static str,
}

const RU: Strings = Strings {
    lang: Lang::Ru,
    app_title: "Ode",
    search_placeholder: "Поиск песен, исполнителей…",
    empty_title: "Ничего не найдено",
    empty_desc: "Попробуйте изменить запрос.",
    error_title: "Ошибка загрузки",
    no_lyrics_title: "Лирика недоступна",
    no_lyrics_desc: "Эта песня не имеет лицензированной лирики в API.",
    open_genius: "Открыть на genius.com",
    copy_lyrics: "Копировать лирику",
    copied: "Скопировано",
    init_search: "Что ищем?",
    search_hint: "Введите название песни или исполнителя",
    settings: "Настройки",
    about: "О приложении",
    clear_cache: "Очистить кэш",
    cache_desc: "Удалить временные ответы API",
    cache_cleared: "Кэш очищен",
    back: "Назад",
    lyrics_attribution: "© Genius",
    search_results: "Результаты поиска",
    select_title: "Выберите песню",
    select_desc: "Найдите песню или исполнителя в поиске",
    annot_title: "Аннотация",
    general_tab: "Основные",
    no_token_title: "Токен не задан",
    no_token_desc: "Сборка выполнена без встроенного токена доступа",
    close: "Закрыть",
    no_lyrics_action: "_Повторить загрузку",
};

const EN: Strings = Strings {
    lang: Lang::En,
    app_title: "Ode",
    search_placeholder: "Search songs, artists…",
    empty_title: "Nothing found",
    empty_desc: "Try a different query.",
    error_title: "Load error",
    no_lyrics_title: "Lyrics unavailable",
    no_lyrics_desc: "This song has no licensed lyrics in the API.",
    open_genius: "Open on genius.com",
    copy_lyrics: "Copy lyrics",
    copied: "Copied",
    init_search: "What to find?",
    search_hint: "Enter a song or artist to search",
    settings: "Settings",
    about: "About",
    clear_cache: "Clear cache",
    cache_desc: "Remove cached API responses",
    cache_cleared: "Cache cleared",
    back: "Back",
    lyrics_attribution: "© Genius",
    search_results: "Search results",
    select_title: "Select a song",
    select_desc: "Find a song or artist in the search",
    annot_title: "Annotation",
    general_tab: "General",
    no_token_title: "No access token",
    no_token_desc: "This build was made without an embedded access token",
    close: "Close",
    no_lyrics_action: "_Retry loading",
};

impl Strings {
    /// Converts an English release date ("May 11, 2023" / "May 2023") to Russian
    pub fn localize_date(&self, d: &str) -> String {
        if self.lang == Lang::En {
            return d.to_string();
        }
        let t = d.trim();
        if t.is_empty() {
            return d.to_string();
        }
        // (English, genitive "D мая", nominative "май")
        const M: &[(&str, &str, &str)] = &[
            ("January", "января", "январь"),
            ("February", "февраля", "февраль"),
            ("March", "марта", "март"),
            ("April", "апреля", "апрель"),
            ("May", "мая", "май"),
            ("June", "июня", "июнь"),
            ("July", "июля", "июль"),
            ("August", "августа", "август"),
            ("September", "сентября", "сентябрь"),
            ("October", "октября", "октябрь"),
            ("November", "ноября", "ноябрь"),
            ("December", "декабря", "декабрь"),
        ];
        let rest = t.trim_start();
        for (en, gi, ni) in M {
            if let Some(after) = rest.strip_prefix(en) {
                let after = after.trim_start().to_string();
                if after.contains(',') && !after.is_empty() {
                    // "11, 2023" -> "11 мая 2023"
                    let comma = after.find(',').unwrap();
                    let day = after[..comma].trim();
                    let year = after[comma + 1..].trim();
                    return format!("{day} {gi} {year}").trim().to_string();
                } else {
                    return format!("{ni} {after}").trim().to_string();
                }
            }
        }
        d.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ru() -> &'static Strings {
        &RU
    }

    #[test]
    fn date_with_day_is_localized() {
        assert_eq!(ru().localize_date("May 11, 2023"), "11 мая 2023");
    }

    #[test]
    fn date_without_day_uses_nominative() {
        assert_eq!(ru().localize_date("May 2023"), "май 2023");
        assert_eq!(ru().localize_date("December 1, 2019"), "1 декабря 2019");
    }

    #[test]
    fn english_dates_are_untouched() {
        assert_eq!(EN.localize_date("May 11, 2023"), "May 11, 2023");
    }

    #[test]
    fn unknown_or_empty_dates_are_untouched() {
        assert_eq!(ru().localize_date("Sometime in 2023"), "Sometime in 2023");
        assert_eq!(ru().localize_date(""), "");
    }
}
