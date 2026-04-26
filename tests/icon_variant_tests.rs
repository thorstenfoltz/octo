//! Tests for the `IconVariant::Random` resolution logic.

use std::collections::HashSet;

use octa::ui::settings::IconVariant;

#[test]
fn random_resolves_to_a_concrete_variant() {
    for _ in 0..50 {
        let resolved = IconVariant::Random.resolve();
        assert_ne!(
            resolved,
            IconVariant::Random,
            "resolve() must never return Random itself"
        );
        assert!(
            IconVariant::CONCRETE.contains(&resolved),
            "resolved variant {:?} not in CONCRETE list",
            resolved
        );
    }
}

#[test]
fn concrete_variants_resolve_to_themselves() {
    for &variant in IconVariant::CONCRETE {
        assert_eq!(
            variant.resolve(),
            variant,
            "concrete variant {:?} must resolve to itself",
            variant
        );
    }
}

#[test]
fn random_distribution_smoke_covers_at_least_eight_colors() {
    // Loose smoke test: with 1000 rolls across 12 buckets, expect very wide
    // coverage. We only assert ≥ 8 distinct hits to keep the test stable.
    let mut seen: HashSet<IconVariant> = HashSet::new();
    for _ in 0..1000 {
        seen.insert(IconVariant::Random.resolve());
    }
    assert!(
        seen.len() >= 8,
        "expected at least 8 distinct colors in 1000 rolls, got {}",
        seen.len()
    );
}

#[test]
fn random_is_first_in_all() {
    // The Random variant should appear first in the combo box list so users
    // discover it without scrolling.
    assert_eq!(IconVariant::ALL.first(), Some(&IconVariant::Random));
}

#[test]
fn all_includes_every_concrete_variant_plus_random() {
    assert_eq!(IconVariant::ALL.len(), IconVariant::CONCRETE.len() + 1);
    for &v in IconVariant::CONCRETE {
        assert!(
            IconVariant::ALL.contains(&v),
            "ALL is missing concrete variant {:?}",
            v
        );
    }
}

#[test]
fn random_svg_source_is_distinct_from_concrete_variants() {
    let random_svg = IconVariant::Random.svg_source();
    for &v in IconVariant::CONCRETE {
        assert_ne!(
            random_svg,
            v.svg_source(),
            "Random SVG should not match the SVG of {:?}",
            v
        );
    }
    assert!(
        random_svg.contains("rainbow") || random_svg.contains("linearGradient"),
        "Random SVG should embed a multi-color gradient"
    );
}

#[test]
fn random_label_is_random() {
    assert_eq!(IconVariant::Random.label(), "Random");
}
