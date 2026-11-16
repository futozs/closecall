use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Case- and diacritic-insensitive form, so `szamla` matches `Számla`.
pub fn fold(text: &str) -> String {
    // Decompose before lowercasing: compatibility forms like `㎒` decompose into uppercase letters.
    text.nfkd()
        .flat_map(char::to_lowercase)
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn strips_hungarian_diacritics_and_case() {
        assert_eq!(fold("Számla"), "szamla");
        assert_eq!(fold("BÉRJEGYZÉK"), "berjegyzek");
        assert_eq!(fold("Árvíztűrő tükörfúrógép"), "arvizturo tukorfurogep");
        assert_eq!(fold("ŐRÜLT ŰR"), "orult ur");
    }

    #[test]
    fn decomposed_input_folds_like_precomposed() {
        assert_eq!(fold("sza\u{301}mla"), fold("számla"));
    }

    #[test]
    fn folds_compatibility_forms() {
        assert_eq!(fold("ＩＮＶＯＩＣＥ"), "invoice");
        assert_eq!(fold("ﬁzetés"), "fizetes");
        assert_eq!(fold("İstanbul"), "istanbul");
        assert_eq!(fold("㎒"), "mhz");
    }

    #[test]
    fn leaves_plain_text_alone() {
        assert_eq!(fold("netbank 2026"), "netbank 2026");
        assert_eq!(fold(""), "");
    }
}
