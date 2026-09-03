use block_native::model::{BinaryOp, Command, Event, Expr, PROJECT_VERSION};
use block_native::parser::{format_project, parse_project};

const SOURCE: &str = r#"
project "demo" {
  stage 480 360 background #f5f7faff
  global score = 0
  list points = [1, 2]
  asset image "cat" = "assets/cat.png"
  asset sound "pop" = "assets/pop.wav"

  sprite "Cat" at 0 0 direction 0 size 32 color #4c97ffff costume "cat" {
    var speed = 4
    list trail = []

    when start {
      repeat 10 {
        move speed * 2 + 1
        change score by 1
        push score to trail
      }
      broadcast "done"
    }

    when key "space" {
      if key("left") and score >= 3 {
        turn -15
      } else {
        call hop(8)
      }
    }

    when message "done" {
      pen down
      while not touching("Cat") {
        move 1
      }
      pen up
      play "pop"
    }

    proc hop(amount) {
      move amount
      wait 0.05
    }
  }
}
"#;

#[test]
fn parses_v2_project_with_events_state_procedures_and_assets() {
    let project = parse_project(SOURCE).expect("source should parse");
    assert_eq!(project.version, PROJECT_VERSION);
    assert_eq!(project.globals[0].name, "score");
    assert_eq!(project.lists[0].items.len(), 2);
    assert_eq!(project.assets.len(), 2);

    let sprite = &project.sprites[0];
    assert_eq!(sprite.costume.as_deref(), Some("cat"));
    assert_eq!(sprite.variables[0].name, "speed");
    assert_eq!(sprite.scripts.len(), 3);
    assert_eq!(sprite.procedures[0].params, vec!["amount"]);
    assert!(matches!(sprite.scripts[0].event, Event::Start));
    assert!(matches!(sprite.scripts[1].event, Event::Key { ref key } if key == "space"));
    assert!(matches!(sprite.scripts[2].event, Event::Message { ref message } if message == "done"));

    let Command::Repeat { body, .. } = &sprite.scripts[0].body[0] else {
        panic!("expected repeat");
    };
    let Command::Move { steps, .. } = &body[0] else {
        panic!("expected move");
    };
    let Expr::Binary {
        op: BinaryOp::Add,
        left,
        ..
    } = steps
    else {
        panic!("expected addition at root");
    };
    assert!(matches!(
        left.as_ref(),
        Expr::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn canonical_formatter_round_trips_v2_semantics() {
    let first = parse_project(SOURCE).unwrap();
    let formatted = format_project(&first);
    let second = parse_project(&formatted).unwrap();

    assert_eq!(first.name, second.name);
    assert_eq!(first.stage, second.stage);
    assert_eq!(first.globals, second.globals);
    assert_eq!(first.lists, second.lists);
    assert_eq!(first.assets, second.assets);
    assert_eq!(first.sprites.len(), second.sprites.len());
    assert_eq!(first.sprites[0].scripts, second.sprites[0].scripts);
    assert_eq!(first.sprites[0].procedures, second.sprites[0].procedures);
}

#[test]
fn version_one_json_upgrades_single_script_to_start_event() {
    let json = r#"{
      "version":1,
      "name":"old",
      "stage":{"width":480,"height":360,"background":[255,255,255,255]},
      "sprites":[{
        "id":"sprite-old","name":"Cat","x":0,"y":0,"direction":0,"size":20,
        "color":[76,151,255,255],
        "script":[{"op":"move","id":"move-old","steps":10}]
      }]
    }"#;

    let project = block_native::model::Project::from_json_str(json).unwrap();
    assert_eq!(project.version, PROJECT_VERSION);
    assert_eq!(project.sprites[0].scripts.len(), 1);
    assert!(matches!(project.sprites[0].scripts[0].event, Event::Start));
    assert!(matches!(
        project.sprites[0].scripts[0].body[0],
        Command::Move { .. }
    ));
}

#[test]
fn parser_reports_source_location_for_invalid_input() {
    let error = parse_project("project \"x\" { stage 480 360 background #ffffffff sprite \"S\" at 0 0 direction 0 size 20 color #ffffffff { when start { if true { move 1 } }")
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("line"));
    assert!(message.contains("column"));
}
