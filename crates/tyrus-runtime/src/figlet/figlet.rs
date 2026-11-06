//! FIGlet ASCII-art rendering for generated tyrus programs, exposed through a C
//! ABI and imported in tyrus as `figlet` from `tyrus:utils`. The font name is
//! validated at compile time; this helper renders the banner using one of the
//! fonts bundled with the `figlet-rs` crate.

use figlet_rs::FIGlet;
use tyrus_string::TyrusString;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_figlet(text: TyrusString, font: TyrusString) -> TyrusString {
    unsafe {
        let text = text_of(&text);
        let font = text_of(&font);
        let banner = render(&text, &font).unwrap_or(text);
        crate::strings::alloc_string(&banner)
    }
}

fn render(text: &str, font: &str) -> Option<String> {
    let figlet = match font {
        "small" => FIGlet::small(),
        "big" => FIGlet::big(),
        "slant" => FIGlet::slant(),
        _ => FIGlet::standard(),
    }
    .ok()?;
    figlet.convert(text).map(|figure| figure.to_string())
}

unsafe fn text_of(value: &TyrusString) -> String {
    if value.is_null() {
        String::new()
    } else {
        unsafe { value.as_str() }.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_banner_and_falls_back_gracefully() {
        assert!(render("Hi", "standard").is_some_and(|art| art.contains('\n')));
        for font in ["standard", "small", "big", "slant"] {
            assert!(render("A", font).is_some(), "font `{font}` should render");
        }
        assert!(render("A", "does-not-exist").is_some());
    }
}
