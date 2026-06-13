## MODIFIED Requirements

### Requirement: Local Settings Web UI
The Local Agent Daemon SHALL serve an HTML user interface on the root path (`GET /`) bound to the local loopback interface (localhost) only, to allow local users to select the active CLI to run.

#### Scenario: Serve settings user interface
- **WHEN** the local user sends a GET request to `/` on the local loopback interface
- **THEN** the system SHALL return a HTTP 200 response with the settings HTML page containing a dropdown to select the active CLI and a Save button
