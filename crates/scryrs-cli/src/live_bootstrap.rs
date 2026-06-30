pub(crate) const SCRYRS_DOCKER_NETWORK_ENV: &str = "SCRYRS_DOCKER_NETWORK";

pub(crate) const SCRYRS_COMPOSE_TEMPLATE: &str = concat!(
    "# Managed by `scryrs init --mode live`. Edit via scryrs, not by hand.\n",
    "version: \"3.8\"\n\n",
    "services:\n",
    "  scryrs:\n",
    "    image: ghcr.io/matthijsrademaker/scryrs-server:latest\n",
    "    container_name: scryrs\n",
    "    restart: unless-stopped\n",
    "    ports:\n",
    "      - \"8081:8081\"\n",
    "    volumes:\n",
    "      - scryrs-data:/data/scryrs\n",
    "    networks:\n",
    "      agent-network:\n",
    "        aliases:\n",
    "          - scryrs\n\n",
    "volumes:\n",
    "  scryrs-data:\n\n",
    "networks:\n",
    "  agent-network:\n",
    "    external: true\n",
    "    name: ${SCRYRS_DOCKER_NETWORK}\n"
);
