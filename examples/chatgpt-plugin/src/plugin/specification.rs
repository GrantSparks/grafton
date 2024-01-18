use std::sync::Arc;

use grafton_server::{
    axum::{routing::get, Router},
    model::AppContext,
    AxumRouter,
};

pub fn build_specification_router(app_ctx: Arc<AppContext>) -> AxumRouter {
    let openapi_yaml = &app_ctx.config.website.pages.with_root().openapi_yaml;

    Router::new().route(openapi_yaml, get(self::get::openapi_handler))
}

mod get {

    use super::*;

    use {
        axum_yaml::Yaml,
        indexmap::IndexMap,
        openapiv3::{
            Components, Info, MediaType, OpenAPI, Operation, PathItem, Paths, ReferenceOr, Schema,
            SchemaData, SchemaKind, Server, StatusCode, Type,
        },
    };

    use grafton_server::{axum::extract::State, model::AppContext};

    pub async fn openapi_handler(_app_ctx: State<Arc<AppContext>>) -> Yaml<OpenAPI> {
        let mut paths = Paths {
            paths: IndexMap::new(),
            extensions: Default::default(),
        };
        paths.paths.insert(
            "/chatgpt-plugin/api/todos".to_string(),
            ReferenceOr::Item(PathItem {
                get: Some(Operation {
                    operation_id: Some("getTodos".to_string()),
                    summary: Some("Get the list of todos".to_string()),
                    responses: openapiv3::Responses {
                        default: None,
                        responses: {
                            let mut map = IndexMap::new();
                            map.insert(
                                StatusCode::Code(200),
                                ReferenceOr::Item(openapiv3::Response {
                                    description: "OK".to_string(),
                                    content: {
                                        let mut map = IndexMap::new();
                                        map.insert(
                                            "application/json".to_string(),
                                            MediaType {
                                                schema: Some(ReferenceOr::Reference {
                                                    reference:
                                                        "#/components/schemas/getTodosResponse"
                                                            .to_string(),
                                                }),
                                                ..Default::default()
                                            },
                                        );
                                        map
                                    },
                                    ..Default::default()
                                }),
                            );
                            map
                        },
                        extensions: Default::default(),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );

        let components = Components {
            schemas: {
                let mut schemas = IndexMap::new();
                schemas.insert(
                    "getTodosResponse".to_string(),
                    ReferenceOr::Item(Schema {
                        schema_data: SchemaData {
                            title: None,
                            description: Some("The list of todos.".to_string()),
                            read_only: false,
                            write_only: false,
                            default: None,
                            deprecated: false,
                            discriminator: None,
                            example: None,
                            external_docs: None,
                            extensions: Default::default(),
                            ..Default::default()
                        },
                        schema_kind: SchemaKind::Type(Type::Array(openapiv3::ArrayType {
                            items: Some(ReferenceOr::Item(Box::new(openapiv3::Schema {
                                schema_data: SchemaData {
                                    title: None,
                                    description: Some("TODO Item".to_string()),
                                    read_only: false,
                                    write_only: false,
                                    default: None,
                                    deprecated: false,
                                    discriminator: None,
                                    example: None,
                                    external_docs: None,
                                    extensions: Default::default(),
                                    ..Default::default()
                                },
                                schema_kind: SchemaKind::Type(Type::String(Default::default())),
                            }))),
                            min_items: Some(0),
                            max_items: None,
                            unique_items: false,
                        })),
                    }),
                );
                schemas
            },
            responses: Default::default(),
            parameters: Default::default(),
            request_bodies: Default::default(),
            ..Default::default()
        };

        let openapi = OpenAPI {
            openapi: "3.0.1".to_string(),
            info: Info {
                title: "TODO Plugin".to_string(),
                description: Some(
                    "A plugin that allows the user to create and manage a TODO list using ChatGPT."
                        .to_string(),
                ),
                version: "v1".to_string(),
                ..Default::default()
            },
            servers: vec![Server {
                url: "https://localhost:8443".to_string(),
                description: Some("This is a description".to_string()),
                variables: Some(IndexMap::new()),
                extensions: Default::default(),
            }],
            paths,
            components: Some(components),
            ..Default::default()
        };

        Yaml(openapi)
    }
}
