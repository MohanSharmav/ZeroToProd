// use tracing::log::Level::Error;
// use unicode_segmentation::UnicodeSegmentation;
//
//
// pub struct NewSubscriber{
//     pub email:String,
//     pub name: SubscriberName,
// }
//
//
// #[derive(Debug)]
// pub struct SubscriberName(pub(crate) String);
//
// impl SubscriberName {
//
//     pub fn parse(s: String) -> Result<SubscriberName,String> {
//     let is_empty_or_whitespace =s.trim().is_empty();
//
//         let is_too_long= s.graphemes(true).count()>256;
//
//
//         let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
//         let contains_forbidden_characters= s
//             .chars()
//             .any(|g|forbidden_characters.contains(&g));
//
//         if is_empty_or_whitespace|| is_too_long || contains_forbidden_characters{
//             Err(format!("{} is not a valid subscriber name",s))
//         }else{
//            Ok(Self(s))
//         }
//
//     }
// }
//
//
//
