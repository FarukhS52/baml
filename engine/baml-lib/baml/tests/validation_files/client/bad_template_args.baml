client<l> MyClient2 {
  provider baml-openai-chat
}

// error: Error validating: This line is invalid. It does not start with any known Baml schema keyword.
//   -->  client/bad_template_args.baml:1
//    | 
//    | 
//  1 | client<l> MyClient2 {
//  2 |   provider baml-openai-chat
//    | 
