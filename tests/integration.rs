use samskara_codegen::SchemaGenerator;

/// Load the samskara-world schema into an in-memory CozoDB, then generate
/// the .capnp output and verify it's deterministic and well-formed.
#[test]
fn full_pipeline_samskara_world() {
    let db = criome_cozo::CriomeDb::open_memory().expect("open memory db");

    // Load the samskara-world-init schema
    let schema_script = include_str!("../../Mentci/Core/samskara-world-init.cozo");
    for stmt in criome_cozo::split_cozo_statements(schema_script) {
        let trimmed = stmt.trim();
        if trimmed.is_empty() || is_comment_only(trimmed) {
            continue;
        }
        db.run_script(trimmed).expect("load schema statement");
    }

    // Seed phase_vocab so vocab detection can query rows
    db.run_script(
        r#"?[name, glyph, in_world_hash, description] <- [
            ["sol", "☉", true, "Manifest — committed truth"],
            ["luna", "☽", false, "Becoming — staged, proposed"],
            ["saturnus", "♄", false, "Archived — superseded, retained"]
        ]
        :put Phase {name => glyph, in_world_hash, description}"#,
    )
    .expect("seed Phase");

    // Seed dignity_vocab
    db.run_script(
        r#"?[name, rank, description] <- [
            ["domicile", 0, "Foundational invariant"],
            ["exaltation", 1, "Verified through trusted source"],
            ["peregrine", 2, "Learned through observation"],
            ["detriment", 3, "Unverified claim"],
            ["fall", 4, "External web source"]
        ]
        :put Dignity {name => rank, description}"#,
    )
    .expect("seed Dignity");

    // Generate schema
    let schema = SchemaGenerator::from_db(&db).expect("from_db");

    // Should have detected phase_vocab and dignity_vocab as enums
    assert!(
        schema.enums.iter().any(|e| e.name == "Phase"),
        "should detect Phase enum from phase_vocab"
    );
    assert!(
        schema.enums.iter().any(|e| e.name == "Dignity"),
        "should detect Dignity enum from dignity_vocab"
    );

    // Should have multiple relation structs
    assert!(
        schema.relations.len() >= 10,
        "expected at least 10 non-vocab relations, got {}",
        schema.relations.len()
    );

    // Generate .capnp text
    let capnp_text = schema.to_capnp_text().expect("to_capnp_text");

    // Basic structure checks
    assert!(capnp_text.contains("@0x"), "should have file ID");
    assert!(capnp_text.contains("struct Thought"), "should have Thought struct");
    assert!(capnp_text.contains("struct AgentSession"), "should have AgentSession struct");
    assert!(capnp_text.contains("struct WorldCommit"), "should have WorldCommit struct");
    assert!(capnp_text.contains("enum Phase"), "should have Phase enum");
    assert!(capnp_text.contains("enum Dignity"), "should have Dignity enum");

    // Field naming conventions
    assert!(capnp_text.contains("createdTs"), "should camelCase created_ts");
    assert!(capnp_text.contains("parentId"), "should camelCase parent_id");

    // Determinism: generate twice, compare
    let capnp_text_2 = schema.to_capnp_text().expect("to_capnp_text second time");
    assert_eq!(capnp_text, capnp_text_2, "capnp output must be deterministic");

    // Hash determinism
    let hash_1 = schema.schema_hash().expect("hash 1");
    let hash_2 = schema.schema_hash().expect("hash 2");
    assert_eq!(hash_1, hash_2, "schema hash must be deterministic");

    // Print for manual inspection
    eprintln!("--- Generated .capnp schema ---");
    eprintln!("{capnp_text}");
    eprintln!("--- Schema hash: {hash_1} ---");
}

fn is_comment_only(stmt: &str) -> bool {
    stmt.lines()
        .all(|line| {
            let trimmed = line.trim();
            trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "//"
        })
}
