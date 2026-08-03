//! Port of validators.test.ts from dcastil/tailwind-merge v3.6.0 — the
//! truth tables that drive class-group value acceptance. The engine's
//! arbitrary-value type validators must match these exactly.

use twm_core::values as v;

#[test]
fn is_any() {
    assert!(v::is_any(""));
    assert!(v::is_any("something"));
}

#[test]
fn is_any_non_arbitrary() {
    for v in [
        "test",
        "1234-hello-world",
        "[hello",
        "hello]",
        "[)",
        "(hello]",
    ] {
        assert!(v::is_any_non_arbitrary(v), "{v}");
    }
    for v in ["[test]", "[label:test]", "(test)", "(label:test)"] {
        assert!(!v::is_any_non_arbitrary(v), "{v}");
    }
}

#[test]
fn is_named_container_query() {
    for v in [
        "@container/sidebar",
        "@container-normal/sidebar",
        "@container-size/sidebar",
        "@container/[sidebar]",
        "@container-size/(--sidebar)",
    ] {
        assert!(v::is_named_container_query(v), "{v}");
    }
    for v in [
        "@container",
        "@container-normal",
        "@container-size",
        "@container/",
        "@container-normal/",
        "@container-size/",
        "@container-[size]/sidebar",
        "@container-foo/sidebar",
        "container/sidebar",
        "hover:@container/sidebar",
    ] {
        assert!(!v::is_named_container_query(v), "{v}");
    }
}

#[test]
fn is_arbitrary_family_name() {
    for v in ["[family-name:Open_Sans]", "[family-name:var(--my-font)]"] {
        assert!(v::is_arbitrary_family_name(v), "{v}");
    }
    for v in [
        "[Open_Sans]",
        "[number:400]",
        "[weight:400]",
        "family-name:test",
        "(family-name:test)",
    ] {
        assert!(!v::is_arbitrary_family_name(v), "{v}");
    }
}

#[test]
fn is_arbitrary_image() {
    for v in [
        "[url:var(--my-url)]",
        "[url(something)]",
        "[url:bla]",
        "[image:bla]",
        "[linear-gradient(something)]",
        "[repeating-conic-gradient(something)]",
    ] {
        assert!(v::is_arbitrary_image(v), "{v}");
    }
    for v in ["[var(--my-url)]", "[bla]", "url:2px", "url(2px)"] {
        assert!(!v::is_arbitrary_image(v), "{v}");
    }
}

#[test]
fn is_arbitrary_length() {
    for v in [
        "[3.7%]",
        "[481px]",
        "[19.1rem]",
        "[50vw]",
        "[56vh]",
        "[length:var(--arbitrary)]",
    ] {
        assert!(v::is_arbitrary_length(v), "{v}");
    }
    for v in ["1", "3px", "1d5", "[1]", "[12px", "12px]", "one"] {
        assert!(!v::is_arbitrary_length(v), "{v}");
    }
}

#[test]
fn is_arbitrary_number() {
    for v in ["[number:black]", "[number:bla]", "[number:230]", "[450]"] {
        assert!(v::is_arbitrary_number(v), "{v}");
    }
    for v in ["[2px]", "[bla]", "[black]", "black", "450"] {
        assert!(!v::is_arbitrary_number(v), "{v}");
    }
}

#[test]
fn is_arbitrary_position() {
    for v in ["[position:2px]", "[position:bla]", "[percentage:bla]"] {
        assert!(v::is_arbitrary_position(v), "{v}");
    }
    for v in ["[2px]", "[bla]", "position:2px"] {
        assert!(!v::is_arbitrary_position(v), "{v}");
    }
}

#[test]
fn is_arbitrary_shadow() {
    for v in [
        "[0_35px_60px_-15px_rgba(0,0,0,0.3)]",
        "[inset_0_1px_0,inset_0_-1px_0]",
        "[0_0_#00f]",
        "[.5rem_0_rgba(5,5,5,5)]",
        "[-.5rem_0_#123456]",
        "[0.5rem_-0_#123456]",
        "[0.5rem_-0.005vh_#123456]",
        "[0.5rem_-0.005vh]",
    ] {
        assert!(v::is_arbitrary_shadow(v), "{v}");
    }
    for v in ["[rgba(5,5,5,5)]", "[#00f]", "[something-else]"] {
        assert!(!v::is_arbitrary_shadow(v), "{v}");
    }
}

#[test]
fn is_arbitrary_weight() {
    for v in [
        "[weight:400]",
        "[weight:bold]",
        "[number:400]",
        "[number:var(--my-weight)]",
        "[400]",
        "[bold]",
    ] {
        assert!(v::is_arbitrary_weight(v), "{v}");
    }
    for v in ["[family-name:test]", "weight:400", "(weight:400)"] {
        assert!(!v::is_arbitrary_weight(v), "{v}");
    }
}

#[test]
fn is_arbitrary_size() {
    for v in ["[size:2px]", "[size:bla]", "[length:bla]"] {
        assert!(v::is_arbitrary_size(v), "{v}");
    }
    for v in ["[2px]", "[bla]", "size:2px", "[percentage:bla]"] {
        assert!(!v::is_arbitrary_size(v), "{v}");
    }
}

#[test]
fn is_arbitrary_value() {
    for v in [
        "[1]",
        "[bla]",
        "[not-an-arbitrary-value?]",
        "[auto,auto,minmax(0,1fr),calc(100vw-50%)]",
    ] {
        assert!(v::is_arbitrary_value(v), "{v}");
    }
    for v in ["[]", "[1", "1]", "1", "one", "o[n]e"] {
        assert!(!v::is_arbitrary_value(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable() {
    for v in [
        "(1)",
        "(bla)",
        "(not-an-arbitrary-value?)",
        "(--my-arbitrary-variable)",
        "(label:--my-arbitrary-variable)",
    ] {
        assert!(v::is_arbitrary_variable(v), "{v}");
    }
    for v in ["()", "(1", "1)", "1", "one", "o(n)e"] {
        assert!(!v::is_arbitrary_variable(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_family_name() {
    assert!(v::is_arbitrary_variable_family_name("(family-name:test)"));
    for v in ["(other:test)", "(test)", "family-name:test"] {
        assert!(!v::is_arbitrary_variable_family_name(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_image() {
    for v in ["(image:test)", "(url:test)"] {
        assert!(v::is_arbitrary_variable_image(v), "{v}");
    }
    for v in ["(other:test)", "(test)", "image:test"] {
        assert!(!v::is_arbitrary_variable_image(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_length() {
    assert!(v::is_arbitrary_variable_length("(length:test)"));
    for v in ["(other:test)", "(test)", "length:test"] {
        assert!(!v::is_arbitrary_variable_length(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_position() {
    assert!(v::is_arbitrary_variable_position("(position:test)"));
    for v in ["(other:test)", "(test)", "position:test", "percentage:test"] {
        assert!(!v::is_arbitrary_variable_position(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_shadow() {
    for v in ["(shadow:test)", "(test)"] {
        assert!(v::is_arbitrary_variable_shadow(v), "{v}");
    }
    for v in ["(other:test)", "shadow:test"] {
        assert!(!v::is_arbitrary_variable_shadow(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_size() {
    for v in ["(size:test)", "(length:test)"] {
        assert!(v::is_arbitrary_variable_size(v), "{v}");
    }
    for v in ["(other:test)", "(test)", "size:test", "(percentage:test)"] {
        assert!(!v::is_arbitrary_variable_size(v), "{v}");
    }
}

#[test]
fn is_arbitrary_variable_weight() {
    for v in ["(weight:test)", "(number:test)", "(--my-weight)"] {
        assert!(v::is_arbitrary_variable_weight(v), "{v}");
    }
    for v in ["(other:test)", "weight:test", "[weight:test]"] {
        assert!(!v::is_arbitrary_variable_weight(v), "{v}");
    }
}

#[test]
fn is_fraction() {
    for v in ["1/2", "123/209"] {
        assert!(v::is_fraction(v), "{v}");
    }
    for v in ["1", "1/2/3", "[1/2]"] {
        assert!(!v::is_fraction(v), "{v}");
    }
}

#[test]
fn is_integer() {
    for v in ["1", "123", "8312"] {
        assert!(v::is_integer(v), "{v}");
    }
    for v in [
        "[8312]",
        "[2]",
        "[8312px]",
        "[8312%]",
        "[8312rem]",
        "8312.2",
        "1.2",
        "one",
        "1/2",
        "1%",
        "1px",
    ] {
        assert!(!v::is_integer(v), "{v}");
    }
}

#[test]
fn is_number() {
    for v in ["1", "123", "8312", "8312.2", "1.2"] {
        assert!(v::is_number(v), "{v}");
    }
    for v in [
        "[8312]",
        "[2]",
        "[8312px]",
        "[8312%]",
        "[8312rem]",
        "one",
        "1/2",
        "1%",
        "1px",
    ] {
        assert!(!v::is_number(v), "{v}");
    }
}

#[test]
fn is_percent() {
    for v in ["1%", "100.001%", ".01%", "0%"] {
        assert!(v::is_percent(v), "{v}");
    }
    for v in ["0", "one%"] {
        assert!(!v::is_percent(v), "{v}");
    }
}

#[test]
fn is_tshirt_size() {
    for v in [
        "xs", "sm", "md", "lg", "xl", "2xl", "2.5xl", "10xl", "2xs", "2lg",
    ] {
        assert!(v::is_tshirt_size(v), "{v}");
    }
    for v in ["", "hello", "1", "xl3", "2xl3", "-xl", "[sm]"] {
        assert!(!v::is_tshirt_size(v), "{v}");
    }
}
