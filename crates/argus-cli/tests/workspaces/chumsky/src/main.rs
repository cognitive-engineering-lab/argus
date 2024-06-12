mod json;

fn main() {}

// use std::collections::HashMap;
// #[derive(Clone, Debug)]
// enum Json {
//   Invalid,
//   Null,
//   Bool(bool),
//   Str(String),
//   Num(f64),
//   Array(Vec<Json>),
//   Object(HashMap<String, Json>),
// }

// fn parser() -> impl Parser<char, Json, Error = Simple<char>> {
//   recursive(|value| {
//     let frac = just('.').chain(text::digits(10));

//     let exp = just('e')
//       .or(just('E'))
//       .chain(just('+').or(just('-')).or_not())
//       .chain(text::digits(10));

//     let number = just('-')
//       .or_not()
//       .chain(text::int(10))
//       .chain(frac.or_not().flatten())
//       .chain::<char, _, _>(exp.or_not().flatten())
//       .collect::<String>()
//       .from_str()
//       .unwrapped()
//       .labelled("number");

//     let escape = just('\\').ignore_then(
//       just('\\')
//         .or(just('/'))
//         .or(just('"'))
//         .or(just('b').to('\x08'))
//         .or(just('f').to('\x0C'))
//         .or(just('n').to('\n'))
//         .or(just('r').to('\r'))
//         .or(just('t').to('\t'))
//         .or(
//           just('u').ignore_then(
//             filter(|c: &char| c.is_digit(16))
//               .repeated()
//               .exactly(4)
//               .collect::<String>()
//               .validate(|digits, span, emit| {
//                 char::from_u32(u32::from_str_radix(&digits, 16).unwrap())
//                   .unwrap_or_else(|| {
//                     emit(Simple::custom(span, "invalid unicode character"));
//                     '\u{FFFD}' // unicode replacement character
//                   })
//               }),
//           ),
//         ),
//     );

//     let string = just('"')
//       .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
//       .then_ignore(just('"'))
//       .collect::<String>()
//       .labelled("string");

//     let array = value
//       .clone()
//       .chain(just(',').ignore_then(value.clone()).repeated())
//       .or_not()
//       .flatten()
//       .delimited_by(just('['), just(']'))
//       .map(Json::Array)
//       .labelled("array");

//     let member = string.clone().then_ignore(just(':').padded()).then(value);
//     let object = member
//       .clone()
//       .chain(just(',').padded().ignore_then(member).repeated())
//       .or_not()
//       .flatten()
//       .padded()
//       .delimited_by(just('{'), just('}'))
//       .collect::<HashMap<String, Json>>()
//       .map(Json::Object)
//       .labelled("object");

//     just("null")
//       .to(Json::Null)
//       .labelled("null")
//       .or(just("true").to(Json::Bool(true)).labelled("true"))
//       .or(just("false").to(Json::Bool(false)).labelled("false"))
//       .or(number.map(Json::Num))
//       // SWAP THESE LINES
//       .or(string)
//       //.or(string.map(Json::Str))
//       .or(array)
//       .or(object)
//       .recover_with(nested_delimiters('{', '}', [('[', ']')], |_| {
//         Json::Invalid
//       }))
//       .recover_with(nested_delimiters('[', ']', [('{', '}')], |_| {
//         Json::Invalid
//       }))
//       .recover_with(skip_then_retry_until(['}', ']']))
//       .padded()
//   })
//   .then_ignore(end().recover_with(skip_then_retry_until([])))
// }

// fn test() {
//   parser().parse("{ \"a\": 5, b: \"hello, world!\"}").unwrap();
// }
