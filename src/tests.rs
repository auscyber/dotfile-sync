mod parse_vars {
    use crate::util::parse_vars;
    use std::env;

    #[test]
    fn simple_input() {
        let input_text = "$PWD/very cool${USER}";
        let expected_text = format!(
            "{}/very cool{}",
            env::var("PWD").unwrap(),
            env::var("USER").unwrap()
        );
        assert_eq!(expected_text, parse_vars(true, None, input_text).unwrap());
    }
    #[test]
    fn no_vars() {
        let input_text = "Hi well";

        assert_eq!(
            input_text.to_string(),
            parse_vars(true, None, input_text).unwrap()
        );
    }
}

//mod goals {
//    use crate::goals::Goal;
//
//    #[test]
//    fn goal_test() {
//        let mut map = std::collections::HashMap::new();
//        macro_rules! goal {
//        ($name:expr) => {
//               map.insert($name,Goal {
//                enabled: true,
//                links: Vec::new(),
//                required_goals: None,
//            });
//
//        };
//        ($name:expr ,  $($depends:expr),*) => {
//            map.insert($name,Goal {
//                enabled: true,
//                links: Vec::new(),
//                required_goals: Some(vec![ $($depends.into(),)*]),
//            });
//        };
//    }
//
//        goal!("hi", "lol");
//        goal!("lol", "bob", "jim");
//        goal!("jim");
//        goal!("bob");
//        println!()
//    }
//}
