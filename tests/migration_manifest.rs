use std::fs;
use std::path::Path;

#[test]
fn migrations_are_contiguous_and_non_destructive() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut nums = Vec::new();
    for entry in fs::read_dir(&dir).expect("migrations") {
        let p = entry.expect("entry").path();
        if p.extension().and_then(|x| x.to_str()) != Some("sql") { continue; }
        let name = p.file_name().unwrap().to_string_lossy();
        let n: u32 = name.split('_').next().unwrap().parse().expect("migration number");
        let sql = fs::read_to_string(&p).expect("sql");
        if n >= 27 {
            assert!(!sql.to_ascii_uppercase().contains("DROP TABLE"), "DROP TABLE interdit dans les migrations S15-S19: {}", name);
            assert!(!sql.to_ascii_uppercase().contains("TRUNCATE"), "TRUNCATE interdit dans les migrations S15-S19: {}", name);
        }
        nums.push(n);
    }
    nums.sort_unstable();
    let expected: Vec<u32> = (1..=nums.len() as u32).collect();
    assert_eq!(nums, expected);
    assert_eq!(*nums.last().unwrap(), 29);
}
