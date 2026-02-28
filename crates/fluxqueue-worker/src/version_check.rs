fn normalize(version: &str) -> String {
    version
        .replace("-rc-", "rc")
        .replace("-beta-", "b")
        .replace("-alpha-", "a")
}

fn parse_part(part: &str) -> (u32, i32, u32) {
    let mut digits = String::new();
    let mut suffix = String::new();

    for c in part.chars() {
        if c.is_ascii_digit() && suffix.is_empty() {
            digits.push(c);
        } else {
            suffix.push(c);
        }
    }

    let number = digits.parse().unwrap_or(0);

    let (ptype, pnum) = if let Some(stripped) = suffix.strip_prefix("rc") {
        (2, stripped.parse().unwrap_or(0))
    } else if let Some(stripped) = suffix.strip_prefix('b') {
        (1, stripped.parse().unwrap_or(0))
    } else if let Some(stripped) = suffix.strip_prefix('a') {
        (0, stripped.parse().unwrap_or(0))
    } else {
        (3, 0) // stable
    };

    (number, ptype, pnum)
}

pub fn compare_versions(v1: &str, v2: &str) -> i8 {
    let v1 = normalize(v1);
    let v2 = normalize(v2);

    let parts1: Vec<_> = v1.split('.').collect();
    let parts2: Vec<_> = v2.split('.').collect();

    let len = parts1.len().max(parts2.len());

    for i in 0..len {
        let p1 = parts1.get(i).unwrap_or(&"0");
        let p2 = parts2.get(i).unwrap_or(&"0");

        let (n1, t1, r1) = parse_part(p1);
        let (n2, t2, r2) = parse_part(p2);

        if n1 != n2 {
            return if n1 > n2 { 1 } else { -1 };
        }

        if t1 != t2 {
            return if t1 > t2 { 1 } else { -1 };
        }

        if r1 != r2 {
            return if r1 > r2 { 1 } else { -1 };
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        let worker_version = "0.3.1";
        let client_lib_version = "0.1.2";

        assert!(
            compare_versions(worker_version, client_lib_version) == 1,
            "left version is greater than the right one"
        );

        assert!(
            compare_versions(client_lib_version, worker_version) == -1,
            "left version is less than the right one"
        );

        assert!(
            compare_versions("0.2.0", "0.2.0") == 0,
            "both are equal versions"
        );

        assert!(
            compare_versions("1.2.0", "1.2") == 0,
            "both are equal versions"
        );

        assert!(
            compare_versions("1.2.3", "1.2.3a1") == 1,
            "alpha version is less than the latest"
        );

        assert!(
            compare_versions("1.2.3", "1.2.3b1") == 1,
            "beta version is less than the latest"
        );

        assert!(
            compare_versions("1.2.3", "1.2.3rc1") == 1,
            "rc version is less than the latest"
        );

        assert!(
            compare_versions("1.2.3-rc-1", "1.2.3rc1") == 0,
            "both are equal"
        );

        assert!(
            compare_versions("1.2.3-beta-1", "1.2.3b1") == 0,
            "both are equal"
        );

        assert!(
            compare_versions("1.2.3-alpha-1", "1.2.3a1") == 0,
            "both are equal"
        );

        assert!(
            compare_versions("1.2.3-rc-1", "1.2.3b1") == 1,
            "left is greater"
        );

        assert!(compare_versions("1.2.3a1", "1.2.3b1") == -1, "alpha < beta");

        assert!(compare_versions("1.2.3b1", "1.2.3rc1") == -1, "beta < rc");

        assert!(compare_versions("1.2.3a1", "1.2.3rc1") == -1, "alpha < rc");

        assert!(compare_versions("1.2.3rc2", "1.2.3rc1") == 1, "rc2 > rc1");

        assert!(compare_versions("1.2.3b2", "1.2.3b10") == -1, "b2 < b10");

        assert!(compare_versions("1.2.3a10", "1.2.3a2") == 1, "a10 > a2");

        assert!(
            compare_versions("1.2.3.0", "1.2.3") == 0,
            "trailing zeros ignored"
        );

        assert!(
            compare_versions("1.2.3.1", "1.2.3") == 1,
            "extra segment greater"
        );

        assert!(
            compare_versions("1.2", "1.2.1") == -1,
            "missing segment treated as zero"
        );

        assert!(
            compare_versions("1.2.3-rc-2", "1.2.3rc1") == 1,
            "rust rc2 > python rc1"
        );

        assert!(
            compare_versions("1.2.3-beta-2", "1.2.3b1") == 1,
            "rust beta2 > python b1"
        );

        assert!(
            compare_versions("1.2.3-alpha-2", "1.2.3a1") == 1,
            "rust alpha2 > python a1"
        );

        assert!(
            compare_versions("1.2.3", "1.2.4a1") == -1,
            "next version prerelease is higher"
        );

        assert!(
            compare_versions("1.2.4a1", "1.2.3") == 1,
            "next version prerelease is higher"
        );

        assert!(
            compare_versions("1.2.3rc1", "1.2.3-rc-1") == 0,
            "symmetry equality"
        );
    }
}
