pub(crate) mod child;
#[cfg(feature = "dev")]
pub(crate) mod dev;
pub(crate) mod mother;
pub(crate) mod scrape;
pub(crate) mod spec;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchFamily {
    Mother,
    Spec,
    Scrape,
    Measure,
    Project,
    Other,
}

#[cfg(test)]
pub(crate) fn classify(command: Option<&crate::Commands>) -> DispatchFamily {
    match command {
        Some(crate::Commands::Mother { .. }) | Some(crate::Commands::Serve { .. }) => {
            DispatchFamily::Mother
        }
        Some(crate::Commands::Spec { .. }) => DispatchFamily::Spec,
        Some(
            crate::Commands::Scrape { .. }
            | crate::Commands::Oxidize { .. }
            | crate::Commands::Rebuild { .. }
            | crate::Commands::Scry { .. }
            | crate::Commands::Context { .. }
            | crate::Commands::Eval { .. }
            | crate::Commands::Bench { .. }
            | crate::Commands::Assay { .. },
        ) => DispatchFamily::Scrape,
        Some(crate::Commands::Measure { .. }) => DispatchFamily::Measure,
        Some(crate::Commands::Init { .. }) => DispatchFamily::Project,
        _ => DispatchFamily::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, DispatchFamily};
    use clap::Parser;

    #[test]
    fn routes_mother_family() {
        let cli = crate::Cli::try_parse_from(["patina", "mother", "status"]).expect("parse");
        assert_eq!(classify(cli.command.as_ref()), DispatchFamily::Mother);
    }

    #[test]
    fn routes_spec_family() {
        let cli = crate::Cli::try_parse_from(["patina", "spec", "list"]).expect("parse");
        assert_eq!(classify(cli.command.as_ref()), DispatchFamily::Spec);
    }

    #[test]
    fn routes_scrape_family() {
        let cli = crate::Cli::try_parse_from(["patina", "scrape", "layer"]).expect("parse");
        assert_eq!(classify(cli.command.as_ref()), DispatchFamily::Scrape);
    }

    #[test]
    fn routes_measure_family() {
        let cli = crate::Cli::try_parse_from(["patina", "measure", "--json"]).expect("parse");
        assert_eq!(classify(cli.command.as_ref()), DispatchFamily::Measure);
    }

    #[test]
    fn routes_project_family() {
        let cli = crate::Cli::try_parse_from(["patina", "init", "demo", "--local", "--no-commit"])
            .expect("parse");
        assert_eq!(classify(cli.command.as_ref()), DispatchFamily::Project);
    }
}
