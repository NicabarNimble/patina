use anyhow::Result;
use duckdb::Connection;
use std::path::Path;

/// Analyze naming patterns from the extracted data
pub fn analyze_naming_patterns(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;
    
    println!("🔍 Discovering Naming Patterns...\n");
    
    // Discover function prefixes (adaptive, not hardcoded!)
    let prefix_query = r#"
        WITH prefixes AS (
            SELECT 
                CASE 
                    WHEN position('_' in name) > 0 THEN 
                        substring(name, 1, position('_' in name))
                    ELSE 
                        substring(name, 1, 
                            CASE 
                                WHEN regexp_matches(name, '[a-z][A-Z]') THEN 
                                    position(regexp_extract(name, '[a-z][A-Z]', 0) in name) + 1
                                ELSE length(name) 
                            END
                        )
                END as prefix,
                COUNT(*) as count
            FROM function_facts
            WHERE length(name) > 2
            GROUP BY prefix
            HAVING COUNT(*) > 5
        )
        SELECT * FROM prefixes
        ORDER BY count DESC
        LIMIT 20
    "#;
    
    let mut stmt = conn.prepare(prefix_query)?;
    let prefix_results = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    
    println!("**Function Prefixes Found:**");
    for result in prefix_results {
        let (prefix, count) = result?;
        println!("  • {} ({} occurrences)", prefix, count);
    }
    
    // Analyze parameter patterns
    let param_query = r#"
        SELECT parameters, COUNT(*) as count
        FROM function_facts  
        WHERE parameters IS NOT NULL
        GROUP BY parameters
        HAVING COUNT(*) > 3
        ORDER BY count DESC
        LIMIT 10
    "#;
    
    let mut stmt = conn.prepare(param_query)?;
    let param_results = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    
    println!("\n**Common Parameter Patterns:**");
    for result in param_results {
        let (params, count) = result?;
        if params.len() < 100 {  // Skip very long parameter lists
            println!("  • {} ({} times)", params, count);
        }
    }
    
    Ok(())
}

/// Analyze code conventions from the extracted data
pub fn analyze_conventions(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;
    
    println!("📐 Analyzing Code Conventions...\n");
    
    // Check error handling patterns
    let error_query = r#"
        SELECT 
            CASE 
                WHEN return_type LIKE '%Result%' THEN 'Result-based'
                WHEN return_type LIKE '%Option%' THEN 'Option-based'
                WHEN return_type LIKE '%Error%' THEN 'Error-type'
                WHEN return_type = 'bool' THEN 'Boolean'
                ELSE 'Other'
            END as error_pattern,
            COUNT(*) as count
        FROM function_facts
        GROUP BY error_pattern
        ORDER BY count DESC
    "#;
    
    let mut stmt = conn.prepare(error_query)?;
    let results = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    
    println!("**Error Handling Patterns:**");
    let mut total = 0;
    let mut patterns = Vec::new();
    for result in results {
        let (pattern, count) = result?;
        patterns.push((pattern, count));
        total += count;
    }
    
    for (pattern, count) in patterns {
        let percentage = (count as f64 / total as f64) * 100.0;
        println!("  • {}: {:.1}% ({} functions)", pattern, percentage, count);
    }
    
    // Check async patterns
    let async_query = r#"
        SELECT 
            COUNT(CASE WHEN is_async = true THEN 1 END) as async_count,
            COUNT(*) as total
        FROM function_facts
    "#;
    
    let (async_count, total): (i32, i32) = conn.query_row(
        async_query,
        [],
        |row| Ok((row.get(0)?, row.get(1)?))
    )?;
    
    if total > 0 {
        let async_percentage = (async_count as f64 / total as f64) * 100.0;
        println!("\n**Async Usage:**");
        println!("  • {:.1}% of functions are async ({}/{})", 
                 async_percentage, async_count, total);
    }
    
    Ok(())
}

/// Analyze architectural patterns
pub fn analyze_architecture(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;
    
    println!("🏗️  Analyzing Architecture...\n");
    
    // Analyze file organization
    let layer_query = r#"
        WITH file_layers AS (
            SELECT 
                CASE 
                    WHEN file LIKE '%/handlers/%' THEN 'handlers'
                    WHEN file LIKE '%/services/%' THEN 'services'
                    WHEN file LIKE '%/models/%' THEN 'models'
                    WHEN file LIKE '%/controllers/%' THEN 'controllers'
                    WHEN file LIKE '%/views/%' THEN 'views'
                    WHEN file LIKE '%/utils/%' THEN 'utils'
                    WHEN file LIKE '%/lib/%' THEN 'lib'
                    WHEN file LIKE '%/src/%' THEN 'src'
                    WHEN file LIKE '%/test%' THEN 'tests'
                    ELSE 'other'
                END as layer,
                COUNT(DISTINCT file) as file_count,
                COUNT(*) as function_count
            FROM function_facts
            GROUP BY layer
            HAVING file_count > 0
        )
        SELECT * FROM file_layers
        ORDER BY function_count DESC
    "#;
    
    let mut stmt = conn.prepare(layer_query)?;
    let results = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, 
            row.get::<_, i32>(1)?,
            row.get::<_, i32>(2)?
        ))
    })?;
    
    println!("**Code Organization Layers:**");
    for result in results {
        let (layer, files, functions) = result?;
        println!("  • {}: {} files, {} functions", layer, files, functions);
    }
    
    // Analyze call patterns
    let call_pattern_query = r#"
        WITH call_sequences AS (
            SELECT 
                caller || ' → ' || callee as pattern,
                COUNT(*) as occurrences
            FROM call_graph
            GROUP BY pattern
            HAVING COUNT(*) > 10
            ORDER BY occurrences DESC
            LIMIT 10
        )
        SELECT * FROM call_sequences
    "#;
    
    let mut stmt = conn.prepare(call_pattern_query)?;
    let results = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    
    println!("\n**Common Call Patterns:**");
    for result in results {
        let (pattern, count) = result?;
        if !pattern.contains("MULT_DIV") && !pattern.contains("Uint8") {  // Skip noise
            println!("  • {} ({} times)", pattern, count);
        }
    }
    
    Ok(())
}

/// Analyze error handling patterns
pub fn analyze_error_handling(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;
    
    println!("⚠️  Analyzing Error Handling...\n");
    
    // Find error handling functions
    let error_functions_query = r#"
        SELECT name, return_type, file
        FROM function_facts
        WHERE name LIKE '%error%' 
           OR name LIKE '%Error%'
           OR return_type LIKE '%Error%'
           OR return_type LIKE '%Result%'
        LIMIT 20
    "#;
    
    let mut stmt = conn.prepare(error_functions_query)?;
    let results = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, 
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?
        ))
    })?;
    
    println!("**Error Handling Functions:**");
    for result in results {
        let (name, return_type, _file) = result?;
        println!("  • {} -> {}", name, return_type);
    }
    
    // Find error handling sequences
    let error_sequence_query = r#"
        SELECT 
            c1.callee as error_check,
            c2.callee as error_handle,
            COUNT(*) as occurrences
        FROM call_graph c1
        JOIN call_graph c2 ON c1.caller = c2.caller 
                          AND c1.file = c2.file
                          AND c1.line_number < c2.line_number
        WHERE (c1.callee LIKE '%Error%' OR c1.callee LIKE '%error%')
          AND (c2.callee LIKE '%Log%' OR c2.callee LIKE '%log%' 
               OR c2.callee LIKE '%panic%' OR c2.callee LIKE '%abort%')
        GROUP BY error_check, error_handle
        HAVING COUNT(*) > 5
        ORDER BY occurrences DESC
        LIMIT 10
    "#;
    
    let mut stmt = conn.prepare(error_sequence_query)?;
    let results = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, 
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?
        ))
    })?;
    
    println!("\n**Error Handling Sequences:**");
    for result in results {
        let (check, handle, count) = result?;
        println!("  • {} → {} ({} times)", check, handle, count);
    }
    
    Ok(())
}