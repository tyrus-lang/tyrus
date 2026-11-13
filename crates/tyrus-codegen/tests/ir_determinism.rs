use std::fs;

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

const PROGRAM: &str = r#"
const main = (): void => {
    let alpha: string[] = ["a"];
    let beta: string[] = ["b"];
    let gamma: string[] = ["c"];
    let delta: string[] = ["d"];
    let epsilon: string[] = ["e"];
    const first: i32 = 2;
    const second: i32 = 3;
    const third: i32 = 5;
    const scaled: i32[] = [1, 2, 3].map(value => value * first + second - third);
    console.log(alpha.length + beta.length + gamma.length + delta.length + epsilon.length);
    console.log(scaled.length);
};
"#;

#[test]
fn generated_ir_is_identical_across_compilations() {
    let directory =
        std::env::temp_dir().join(format!("tyrus-ir-determinism-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("could not create the work directory");
    let entry = directory.join("main.tyr");
    fs::write(&entry, PROGRAM).expect("could not write the program");

    let mut previous: Option<String> = None;
    for round in 0..3 {
        let program = module_loader::load_program(&entry, None).expect("the program must load");
        let stem = directory.join(format!("round{round}"));
        codegen::compile(&program, "determinism", &stem.to_string_lossy(), false)
            .expect("the program must compile");
        let ir = fs::read_to_string(format!("{}.ll", stem.display())).expect("missing .ll");
        if let Some(first) = &previous {
            assert_eq!(
                first, &ir,
                "compilation {round} produced different IR for the same source"
            );
        }
        previous = Some(ir);
    }

    fs::remove_dir_all(&directory).ok();
}
