mod api_docs;
mod config;
mod dtos;
mod endpoints;
mod middleware;
mod models;
mod modifiers;
mod modules;
mod otel;
mod router;
mod settings;

use actix_cors::Cors;
use actix_web::{
    dev::Service,
    middleware::{Logger, NormalizePath, TrailingSlash},
    web, App, HttpServer,
};
use actix_web_opentelemetry::RequestTracing;
use api_docs::ApiDoc;
use awc::{error::HeaderValue, Client};
use config::injectors::configure as configure_injection_modules;
use core::panic;
use endpoints::{
    index::heath_check_endpoints,
    manager::{
        account_endpoints as manager_account_endpoints,
        guest_role_endpoints as manager_guest_role_endpoints,
        tenant_endpoints as manager_tenant_endpoints,
    },
    role_scoped::configure as configure_standard_endpoints,
    service::{
        account_endpoints as service_account_endpoints,
        auxiliary_endpoints as service_auxiliary_endpoints,
        guest_endpoints as service_guest_endpoints,
    },
    shared::insert_role_header,
    staff::account_endpoints as staff_account_endpoints,
};
use models::{
    api_config::{LogFormat, LoggingTarget},
    config_handler::ConfigHandler,
};
use myc_config::{
    init_vault_config_from_file, optional_config::OptionalConfig,
};
use myc_core::{domain::dtos::http::Protocol, settings::init_in_memory_routes};
use myc_http_tools::{
    providers::{azure_endpoints, google_endpoints},
    settings::DEFAULT_REQUEST_ID_KEY,
};
use myc_notifier::{
    executor::consume_messages,
    repositories::MessageSendingSmtpRepository,
    settings::{init_queue_config_from_file, init_smtp_config_from_file},
};
use myc_prisma::repositories::connector::generate_prisma_client_of_thread;
use oauth2::http::HeaderName;
use openssl::{
    pkey::PKey,
    ssl::{SslAcceptor, SslMethod},
    x509::X509,
};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use otel::{metadata_from_headers, parse_otlp_headers_from_env};
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_LENGTH, CONTENT_TYPE,
};
use router::route_request;
use settings::{ADMIN_API_SCOPE, GATEWAY_API_SCOPE, SUPER_USER_API_SCOPE};
use std::{
    path::PathBuf, process::id as process_id, str::FromStr, time::Duration,
};
use tracing::{info, trace};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};
use utoipa::OpenApi;
use utoipa_redoc::{FileConfig, Redoc, Servable};
use utoipa_swagger_ui::{oauth, Config, SwaggerUi};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? API fire elements
// ? ---------------------------------------------------------------------------

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    // ? -----------------------------------------------------------------------
    // ? Export the UTOIPA_REDOC_CONFIG_FILE environment variable
    //
    // The UTOIPA_REDOC_CONFIG_FILE environment variable should be exported
    // before the server starts. The variable should contain the path to the
    // redoc configuration file.
    //
    // ? -----------------------------------------------------------------------

    if let Err(err) = std::env::var("UTOIPA_REDOC_CONFIG_FILE") {
        trace!("Error on get env `UTOIPA_REDOC_CONFIG_FILE`: {err}");
        info!("Env variable `UTOIPA_REDOC_CONFIG_FILE` not set. Setting default value");

        std::env::set_var(
            "UTOIPA_REDOC_CONFIG_FILE",
            "ports/api/src/api_docs/redoc.config.json",
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Initialize services configuration
    //
    // All configurations for the core, ports, and adapters layers should exists
    // into the configuration file. Such file are loaded here.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing services configuration");

    let env_config_path = match std::env::var("SETTINGS_PATH") {
        Ok(path) => path,
        Err(err) => panic!("Error on get env `SETTINGS_PATH`: {err}"),
    };

    let config =
        match ConfigHandler::init_from_file(PathBuf::from(env_config_path)) {
            Ok(res) => res,
            Err(err) => panic!("Error on init config: {err}"),
        };

    let api_config = config.api.clone();

    // ? -----------------------------------------------------------------------
    // ? Configure logging and telemetry
    //
    // The logging and telemetry configuration should be initialized before the
    // server starts. The configuration should be set to the server and the
    // server should be started.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing Logging and Telemetry configuration");

    let logging_config = api_config.to_owned().logging;

    let (non_blocking, _guard) = match logging_config.target.to_owned() {
        //
        // If a log file is provided, log to the file
        //
        Some(LoggingTarget::File { path }) => {
            let mut log_file = PathBuf::from(path);

            let binding = log_file.to_owned();
            let parent_dir = binding
                .parent()
                .expect("Log file parent directory not found");

            match logging_config.format {
                LogFormat::Jsonl => {
                    log_file.set_extension("jsonl");
                }
                LogFormat::Ansi => {
                    log_file.set_extension("log");
                }
            };

            if log_file.exists() {
                std::fs::remove_file(&log_file)?;
            }

            let file_name =
                log_file.file_name().expect("Log file name not found");

            let file_appender =
                tracing_appender::rolling::never(parent_dir, file_name);

            tracing_appender::non_blocking(file_appender)
        }
        //
        // If no log file is provided, log to stderr
        //
        _ => tracing_appender::non_blocking(std::io::stderr()),
    };

    if let Some(LoggingTarget::Jaeger {
        name,
        protocol,
        host,
        port,
    }) = logging_config.target
    {
        //
        // Jaeger logging configurations
        //

        std::env::set_var("OTEL_SERVICE_NAME", name.to_owned());
        let headers = parse_otlp_headers_from_env();
        let tracer = opentelemetry_otlp::new_pipeline().tracing();

        let tracer = (match protocol {
            Protocol::Grpc => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(format!("{}://{}:{}", protocol, host, port))
                    .with_metadata(metadata_from_headers(headers));

                tracer.with_exporter(exporter)
            }
            _ => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(format!(
                        "{}://{}:{}/v1/logs",
                        protocol, host, port
                    ))
                    .with_headers(headers.into_iter().collect());

                tracer.with_exporter(exporter)
            }
        })
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to install OpenTelemetry tracer")
        .tracer(name);

        let telemetry_layer =
            tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::Registry::default()
            .with(telemetry_layer)
            .init();
    } else {
        //
        // Default logging configurations
        //

        let tracing_formatting_layer = tracing_subscriber::fmt()
            .event_format(
                fmt::format()
                    .with_level(true)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_source_location(true),
            )
            .with_writer(non_blocking)
            .with_env_filter(
                EnvFilter::from_str(logging_config.level.as_str()).unwrap(),
            );

        match logging_config.format {
            LogFormat::Ansi => tracing_formatting_layer.pretty().init(),
            LogFormat::Jsonl => tracing_formatting_layer.json().init(),
        };
    };

    // ? -----------------------------------------------------------------------
    // ? Routes should be used on API gateway
    //
    // When users perform queries to the API gateway, the gateway should
    // redirect the request to the correct service. Services are loaded into
    // memory and the gateway should know the routes during their execution.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing routes");
    init_in_memory_routes(Some(config.api.routes.clone())).await;

    // ? -----------------------------------------------------------------------
    // ? Initialize vault configuration
    //
    // The vault configuration should be initialized before the server starts.
    // Vault configurations should be used to store sensitive data.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing Vault configs");
    init_vault_config_from_file(None, Some(config.vault)).await;

    // ? -----------------------------------------------------------------------
    // ? Initialize notifier elements
    //
    // SMTP and Queue configurations should be initialized before the server
    // starts. TH QUEUE server should be started to allow queue messages to be
    // consumed. The SMTP server should be started to allow emails to be sent.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing SMTP configs");
    init_smtp_config_from_file(None, Some(config.smtp)).await;

    info!("Initializing QUEUE configs");
    init_queue_config_from_file(None, Some(config.queue.to_owned())).await;

    // ? -----------------------------------------------------------------------
    // ? Here the current thread receives an instance of the prisma client.
    //
    // Each thread should contains a prisma instance. Otherwise the application
    // should raise an adapter error on try to perform the first database query.
    //
    // ? -----------------------------------------------------------------------
    info!("Start the database connectors");

    let database_url = config.prisma.database_url.async_get_or_error().await;

    std::env::set_var(
        "DATABASE_URL",
        match database_url {
            Ok(url) => url,
            Err(err) => panic!("Error on get database url: {err}"),
        },
    );

    generate_prisma_client_of_thread(process_id()).await;

    // ? -----------------------------------------------------------------------
    // ? Fire the scheduler
    // ? -----------------------------------------------------------------------
    info!("Fire mycelium scheduler");

    let queue_config = match config.queue.to_owned() {
        OptionalConfig::Enabled(queue) => queue,
        _ => panic!("Queue config not found"),
    };

    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(
            match queue_config
                .consume_interval_in_secs
                .async_get_or_error()
                .await
            {
                Ok(interval) => interval,
                Err(err) => {
                    panic!("Error on get consume interval: {err}");
                }
            },
        ));

        loop {
            interval.tick().await;
            let queue_name = match queue_config
                .clone()
                .email_queue_name
                .async_get_or_error()
                .await
            {
                Ok(name) => name,
                Err(err) => {
                    panic!("Error on get queue name: {err}");
                }
            };

            match consume_messages(
                queue_name.to_owned(),
                Box::new(&MessageSendingSmtpRepository {}),
            )
            .await
            {
                Ok(messages) => {
                    if messages > 0 {
                        trace!(
                            "'{}' messages consumed from the queue '{}'",
                            messages,
                            queue_name.to_owned()
                        )
                    }
                }
                Err(err) => {
                    if !err.expected() {
                        panic!("Error on consume messages: {err}");
                    }
                }
            };
        }
    });

    // ? -----------------------------------------------------------------------
    // ? Configure the server
    // ? -----------------------------------------------------------------------
    info!("Set the server configuration");
    let server = HttpServer::new(move || {
        let local_api_config = config.api.clone();
        let forward_api_config = config.api.clone();
        let auth_config = config.auth.clone();
        let token_config = config.core.account_life_cycle.clone();

        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _| {
                local_api_config
                    .allowed_origins
                    .contains(&origin.to_str().unwrap_or("").to_string())
            })
            .expose_headers(vec![
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
                ACCESS_CONTROL_ALLOW_METHODS,
                ACCESS_CONTROL_ALLOW_ORIGIN,
                CONTENT_LENGTH,
                CONTENT_TYPE,
                ACCEPT,
                HeaderName::from_str(DEFAULT_REQUEST_ID_KEY).unwrap(),
            ])
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);

        trace!("Configured Cors: {:?}", cors);

        // ? -------------------------------------------------------------------
        // ? Configure base application
        // ? -------------------------------------------------------------------

        let app = App::new()
            .wrap(RequestTracing::new())
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(token_config).clone())
            .app_data(web::Data::new(auth_config.to_owned()).clone())
            //
            // Index
            //
            // Index endpoints allow to check fht status of the service.
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Health)
                        .as_str(),
                )
                .configure(heath_check_endpoints::configure),
            );

        // ? -------------------------------------------------------------------
        // ? Configure base mycelium scope
        // ? -------------------------------------------------------------------
        let mycelium_scope = web::scope(&format!("/{}", ADMIN_API_SCOPE))
            //
            // Super Users
            //
            // Super user endpoints allow to perform manage the staff and
            // manager users actions, including determine new staffs and
            // managers.
            //
            .service(
                web::scope(format!("/{}", SUPER_USER_API_SCOPE).as_str())
                    .service(
                        web::scope(
                            format!("/{}", endpoints::shared::UrlScope::Staffs)
                                .as_str(),
                        )
                        //
                        // Inject a header to be collected by the
                        // MyceliumProfileData extractor.
                        //
                        // An empty role header was injected to allow only the
                        // super users with Staff status to access the staff
                        // endpoints.
                        //
                        .wrap_fn(|req, srv| {
                            let req = insert_role_header(req, vec![]);

                            srv.call(req)
                        })
                        //
                        // Configure endpoints
                        //
                        .configure(staff_account_endpoints::configure),
                    )
                    //
                    // Manager Users
                    //
                    .service(
                        web::scope(
                            format!(
                                "/{}",
                                endpoints::shared::UrlScope::Managers
                            )
                            .as_str(),
                        )
                        //
                        // Inject a header to be collected by the
                        // MyceliumProfileData extractor.
                        //
                        // An empty role header was injected to allow only the
                        // super users with Managers status to access the
                        // managers endpoints.
                        //
                        .wrap_fn(|req, srv| {
                            let req = insert_role_header(req, vec![]);

                            srv.call(req)
                        })
                        //
                        // Configure endpoints
                        //
                        .configure(manager_tenant_endpoints::configure)
                        .configure(manager_guest_role_endpoints::configure)
                        .configure(manager_account_endpoints::configure),
                    ),
            )
            //
            // Role Scoped Endpoints
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::RoleScoped)
                        .as_str(),
                )
                .configure(configure_standard_endpoints),
            )
            //
            // Service Scoped Endpoints
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Service)
                        .as_str(),
                )
                .configure(service_guest_endpoints::configure)
                .configure(service_account_endpoints::configure)
                .configure(service_auxiliary_endpoints::configure),
            );

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Mycelium Auth
        //
        // ? -------------------------------------------------------------------
        let app = match auth_config.internal {
            OptionalConfig::Enabled(config) => {
                //
                // Configure OAuth2 Scope
                //
                info!("Configuring Mycelium Internal authentication");
                app.app_data(web::Data::new(config.clone()))
            }
            _ => app,
        };

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Google OAuth2
        //
        // ? -------------------------------------------------------------------
        let mycelium_scope = match auth_config.google {
            OptionalConfig::Enabled(_) => {
                //
                // Configure OAuth2 Scope
                //
                info!("Configuring Google authentication");
                let scope = mycelium_scope.service(
                    web::scope("/auth/google")
                        .configure(google_endpoints::configure),
                );

                scope
            }
            _ => mycelium_scope,
        };

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Azure AD OAuth2
        //
        // ? -------------------------------------------------------------------
        let mycelium_scope = match auth_config.azure {
            OptionalConfig::Enabled(_) => {
                //
                // Configure OAuth2 Scope
                //
                info!("Configuring Azure AD authentication");
                let scope = mycelium_scope.service(
                    web::scope("/auth/azure")
                        .configure(azure_endpoints::configure),
                );

                scope
            }
            _ => mycelium_scope,
        };

        // ? -------------------------------------------------------------------
        // ? Fire the server
        // ? -------------------------------------------------------------------
        app
            // ? ---------------------------------------------------------------
            // ? Normalize path
            // ? ---------------------------------------------------------------
            .wrap(NormalizePath::new(TrailingSlash::MergeOnly))
            // ? ---------------------------------------------------------------
            // ? Configure CORS policies
            // ? ---------------------------------------------------------------
            .wrap(cors)
            // ? ---------------------------------------------------------------
            // ? Configure Log elements
            // ? ---------------------------------------------------------------
            // These wrap create the basic log elements and exclude the health
            // check route.
            .wrap(
                Logger::default()
                    .exclude_regex("/health/*")
                    .exclude_regex("/doc/swagger/*")
                    .exclude_regex("/doc/redoc/*"),
            )
            // ? ---------------------------------------------------------------
            // ? Configure Injection modules
            // ? ---------------------------------------------------------------
            .configure(configure_injection_modules)
            // ? ---------------------------------------------------------------
            // ? Configure mycelium routes
            // ? ---------------------------------------------------------------
            .service(mycelium_scope)
            // ? ---------------------------------------------------------------
            // ? Configure API documentation
            // ? ---------------------------------------------------------------
            .service(Redoc::with_url_and_config(
                "/doc/redoc",
                ApiDoc::openapi(),
                FileConfig,
            ))
            .service(
                SwaggerUi::new("/doc/swagger/{_:.*}")
                    .url("/doc/openapi.json", ApiDoc::openapi())
                    .oauth(
                        oauth::Config::new()
                            .client_id("client-id")
                            .scopes(vec![String::from("openid")])
                            .use_pkce_with_authorization_code_grant(true),
                    )
                    .config(
                        Config::default()
                            .filter(true)
                            .show_extensions(true)
                            .persist_authorization(true)
                            .show_common_extensions(true)
                            .request_snippets_enabled(true),
                    ),
            )
            // ? ---------------------------------------------------------------
            // ? Configure gateway routes
            // ? ---------------------------------------------------------------
            .app_data(web::Data::new(Client::default()))
            .app_data(web::Data::new(local_api_config.gateway_timeout))
            .app_data(web::Data::new(forward_api_config.to_owned()).clone())
            .service(
                web::scope(&format!("/{}", GATEWAY_API_SCOPE))
                    //
                    // Inject a request ID to downstream services
                    //
                    .wrap_fn(|mut req, srv| {
                        req.headers_mut().insert(
                            HeaderName::from_str(DEFAULT_REQUEST_ID_KEY)
                                .unwrap(),
                            HeaderValue::from_str(
                                Uuid::new_v4().to_string().as_str(),
                            )
                            .unwrap(),
                        );

                        srv.call(req)
                    })
                    //
                    // Route to default route
                    //
                    .default_service(web::to(route_request)),
            )
    });

    let address = (
        api_config.to_owned().service_ip,
        api_config.to_owned().service_port,
    );

    info!("Listening on Address and Port: {:?}: ", address);

    if let OptionalConfig::Enabled(tls_config) = api_config.to_owned().tls {
        let api_config = api_config.clone();

        info!("Load TLS cert and key");

        // To create a self-signed temporary cert for testing:
        //
        // openssl req
        //     -x509 \
        //     -newkey rsa:4096 \
        //     -nodes \
        //     -keyout key.pem \
        //     -out cert.pem \
        //     -days 365 \
        //     -subj '/CN=localhost'
        //
        let mut builder =
            SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

        //
        // Read the certificate content
        //
        let cert_pem = match tls_config.tls_cert.async_get_or_error().await {
            Ok(path) => path,
            Err(err) => panic!("Error on get TLS cert path: {err}"),
        };

        let cert = X509::from_pem(cert_pem.as_bytes())?;

        //
        // Read the certificate key
        //
        let key_pem = match tls_config.tls_key.async_get_or_error().await {
            Ok(path) => path,
            Err(err) => panic!("Error on get TLS key path: {err}"),
        };

        let key = PKey::private_key_from_pem(key_pem.as_bytes())?;

        //
        // Set the certificate and key
        //
        builder.set_certificate(&cert).unwrap();
        builder.set_private_key(&key).unwrap();

        info!("Fire the server with TLS");
        return server
            .bind_openssl(format!("{}:{}", address.0, address.1), builder)?
            .workers(api_config.service_workers.to_owned() as usize)
            .run()
            .await;
    }

    info!("Fire the server without TLS");
    server
        .bind(address)?
        .workers(api_config.service_workers as usize)
        .run()
        .await
}
