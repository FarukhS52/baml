use crate::validate::validation_pipeline::context::Context;

use internal_baml_diagnostics::{DatamodelError, DatamodelWarning, Span};

use internal_baml_schema_ast::ast::{WithIdentifier, WithName, WithSpan};

use super::types::validate_type;

pub(super) fn validate(ctx: &mut Context<'_>) {
    let clients = ctx
        .db
        .walk_clients()
        .map(|c| c.name().to_string())
        .collect::<Vec<_>>();

    let mut defined_types = internal_baml_jinja::PredefinedTypes::default();
    ctx.db.walk_classes().for_each(|t| {
        t.add_to_types(&mut defined_types);
    });
    ctx.db.walk_templates().for_each(|t| {
        t.add_to_types(&mut defined_types);
    });

    // Validate template strings
    for template in ctx.db.walk_templates() {
        let prompt = match template.template_raw() {
            Some(p) => p,
            None => {
                ctx.push_error(DatamodelError::new_validation_error(
                    "Template string must be a raw string literal like `template_string MyTemplate(myArg: string) #\"\n\n\"#`",
                    template.identifier().span().clone(),
                ));
                continue;
            }
        };

        defined_types.start_scope();
        if let Some(p) = template.ast_node().input() {
            p.args.iter().for_each(|(name, t)| {
                defined_types.add_variable(name.name(), ctx.db.to_jinja_type(&t.field_type))
            });
        }
        match internal_baml_jinja::validate_template(
            template.name(),
            prompt.raw_value(),
            &mut defined_types,
        ) {
            Ok(_) => {}
            Err(e) => {
                let pspan = prompt.span();
                if let Some(_e) = e.parsing_errors {
                    // ctx.push_error(DatamodelError::new_validation_error(
                    //     &format!("Error parsing jinja template: {}", e),
                    //     e.line(),
                    // ))
                } else {
                    e.errors.iter().for_each(|t| {
                        let span = t.span();
                        let span = Span::new(
                            pspan.file.clone(),
                            pspan.start + span.start_offset as usize,
                            pspan.start + span.end_offset as usize,
                        );
                        ctx.push_warning(DatamodelWarning::new(t.message().to_string(), span))
                    })
                }
            }
        }
        defined_types.end_scope();
        defined_types.errors_mut().clear();
    }

    for func in ctx.db.walk_functions() {
        for args in func.walk_input_args().chain(func.walk_output_args()) {
            let arg = args.ast_arg();
            validate_type(ctx, &arg.1.field_type)
        }

        // Ensure the client is correct.
        // TODO: message to the user that it should be either a client ref OR an inline client
        match func.client_spec() {
            Ok(_) => {}
            Err(e) => {
                let client = match func.metadata().client.as_ref() {
                    Some(client) => client,
                    None => {
                        ctx.push_error(DatamodelError::new_validation_error(
                            "Client metadata is missing.",
                            func.span().clone(),
                        ));
                        continue;
                    }
                };
                ctx.push_error(DatamodelError::not_found_error(
                    "Client",
                    &client.0,
                    client.1.clone(),
                    clients.clone(),
                ))
            }
        }

        let prompt = match func.metadata().prompt.as_ref() {
            Some(prompt) => prompt,
            None => {
                ctx.push_error(DatamodelError::new_validation_error(
                    "Prompt metadata is missing.",
                    func.span().clone(),
                ));
                continue;
            }
        };
        defined_types.start_scope();

        func.walk_input_args().for_each(|arg| {
            let name = match arg.ast_arg().0 {
                Some(arg) => arg.name().to_string(),
                None => {
                    ctx.push_error(DatamodelError::new_validation_error(
                        "Argument name is missing.",
                        arg.ast_arg().1.span().clone(),
                    ));
                    return;
                }
            };

            let field_type = ctx.db.to_jinja_type(&arg.ast_arg().1.field_type);

            defined_types.add_variable(&name, field_type);
        });
        match internal_baml_jinja::validate_template(
            func.name(),
            prompt.raw_value(),
            &mut defined_types,
        ) {
            Ok(_) => {}
            Err(e) => {
                let pspan = prompt.span();
                if let Some(e) = e.parsing_errors {
                    // ctx.push_error(DatamodelError::new_validation_error(
                    //     &format!("Error parsing jinja template: {}", e),
                    //     // e.,
                    // ))
                } else {
                    e.errors.iter().for_each(|t| {
                        let span = t.span();
                        let span = Span::new(
                            pspan.file.clone(),
                            pspan.start + span.start_offset as usize,
                            pspan.start + span.end_offset as usize,
                        );
                        ctx.push_warning(DatamodelWarning::new(t.message().to_string(), span))
                    })
                }
            }
        }
        defined_types.end_scope();
        defined_types.errors_mut().clear();
    }
}
