//! Declarative tool definitions for the pcli2 MCP server.
//!
//! Every MCP tool is described by a [`ToolSpec`]: the pcli2 subcommand it
//! runs and the arguments it accepts. The same table drives both the JSON
//! schema advertised through `tools/list` and the argv construction used by
//! `tools/call`, so the two can never drift apart.

use serde_json::{Map, Value, json};

/// How a JSON argument is turned into pcli2 command-line arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Boolean; emits the flag when true.
    Flag,
    /// String; emits `flag value`.
    Str,
    /// Floating point number; emits `flag value`.
    Num,
    /// Unsigned integer; emits `flag value`.
    Int,
    /// String, comma-separated string, or array of strings; emits `flag value` once per item.
    StrList,
    /// String emitted as a bare positional argument.
    Positional,
    /// Present in the schema only; handled by the server, never forwarded to pcli2.
    Local,
}

#[derive(Clone, Copy, Debug)]
pub struct ArgSpec {
    pub key: &'static str,
    pub flag: &'static str,
    pub kind: Kind,
    pub description: &'static str,
    pub enum_values: &'static [&'static str],
    pub range: Option<(f64, f64)>,
    pub required: bool,
}

impl ArgSpec {
    pub const fn new(
        key: &'static str,
        flag: &'static str,
        kind: Kind,
        description: &'static str,
    ) -> Self {
        Self {
            key,
            flag,
            kind,
            description,
            enum_values: &[],
            range: None,
            required: false,
        }
    }

    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub const fn values(mut self, values: &'static [&'static str]) -> Self {
        self.enum_values = values;
        self
    }

    pub const fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    pub const fn describe(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }
}

pub const fn flag(key: &'static str, flag: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, flag, Kind::Flag, description)
}

pub const fn string(key: &'static str, flag: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, flag, Kind::Str, description)
}

pub const fn number(key: &'static str, flag: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, flag, Kind::Num, description)
}

pub const fn integer(key: &'static str, flag: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, flag, Kind::Int, description)
}

pub const fn string_list(
    key: &'static str,
    flag: &'static str,
    description: &'static str,
) -> ArgSpec {
    ArgSpec::new(key, flag, Kind::StrList, description)
}

pub const fn positional(key: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, "", Kind::Positional, description)
}

pub const fn local(key: &'static str, description: &'static str) -> ArgSpec {
    ArgSpec::new(key, "", Kind::Local, description)
}

#[derive(Clone, Copy, Debug)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// pcli2 arguments placed before the tool arguments, e.g. `["asset", "get"]`.
    pub command: &'static [&'static str],
    pub args: &'static [ArgSpec],
    /// Groups of argument keys of which at least one must be supplied.
    pub one_of: &'static [&'static [&'static str]],
}

const fn tool(
    name: &'static str,
    description: &'static str,
    command: &'static [&'static str],
    args: &'static [ArgSpec],
    one_of: &'static [&'static [&'static str]],
) -> ToolSpec {
    ToolSpec {
        name,
        description,
        command,
        args,
        one_of,
    }
}

// ---------------------------------------------------------------------------
// Shared argument definitions
// ---------------------------------------------------------------------------

const TENANT: ArgSpec = string(
    "tenant",
    "--tenant",
    "Tenant ID or alias. Defaults to the active tenant.",
);
const HEADERS: ArgSpec = flag(
    "headers",
    "--headers",
    "Include column headers in the output.",
);
const PRETTY: ArgSpec = flag("pretty", "--pretty", "Pretty-print the output.");
const METADATA: ArgSpec = flag(
    "metadata",
    "--metadata",
    "Include asset metadata in the output.",
);
const FORMAT_JC: ArgSpec =
    string("format", "--format", "Output format. Default json.").values(&["json", "csv"]);
const FORMAT_JCT: ArgSpec =
    string("format", "--format", "Output format. Default json.").values(&["json", "csv", "tree"]);
const YES: ArgSpec = flag(
    "yes",
    "--yes",
    "Automatically confirm any confirmation prompt. Without it a command that needs confirmation fails.",
);
const DRY_RUN: ArgSpec = flag(
    "dry_run",
    "--dry-run",
    "Show what would be done without making any changes.",
);
const PROGRESS: ArgSpec = flag(
    "progress",
    "--progress",
    "Display a progress bar during processing (written to stderr).",
);
const CONCURRENT: ArgSpec = integer(
    "concurrent",
    "--concurrent",
    "Maximum number of concurrent operations (1-10). Default 1.",
)
.range(1.0, 10.0);
const DELAY: ArgSpec = integer(
    "delay",
    "--delay",
    "Delay in seconds between operations (0-180). Default 0.",
)
.range(0.0, 180.0);
const CONTINUE_ON_ERROR: ArgSpec = flag(
    "continue_on_error",
    "--continue-on-error",
    "Continue processing the remaining items if one fails.",
);
const RELOAD: ArgSpec = flag(
    "reload",
    "--reload",
    "Force refresh of the folder cache from the API first.",
);

const UUID: ArgSpec = string("uuid", "--uuid", "Asset UUID.");
const PATH: ArgSpec = string("path", "--path", "Asset path, e.g. /Root/Folder/Asset.stl.");
const UUID_OR_PATH: &[&[&str]] = &[&["uuid", "path"]];

const FOLDER_UUID: ArgSpec = string("folder_uuid", "--folder-uuid", "Folder UUID.");
const FOLDER_PATH: ArgSpec = string(
    "folder_path",
    "--folder-path",
    "Folder path, e.g. /Root/Child/Grandchild.",
);
const FOLDER_UUID_OR_PATH: &[&[&str]] = &[&["folder_uuid", "folder_path"]];
const FOLDER_PATHS: ArgSpec = string_list(
    "folder_path",
    "--folder-path",
    "Folder path(s) to process: a string, a comma-separated string, or an array of strings.",
)
.required();

const PARENT_FOLDER_UUID: ArgSpec = string(
    "parent_folder_uuid",
    "--parent-folder-uuid",
    "Parent folder UUID.",
);
const PARENT_FOLDER_PATH: ArgSpec = string(
    "parent_folder_path",
    "--parent-folder-path",
    "Parent folder path, e.g. /Root/Child.",
);
const PARENT_ONE_OF: &[&str] = &["parent_folder_uuid", "parent_folder_path"];

const REFERENCE_UUID: ArgSpec = string(
    "reference_uuid",
    "--reference-uuid",
    "Reference (source) asset UUID.",
);
const REFERENCE_PATH: ArgSpec = string(
    "reference_path",
    "--reference-path",
    "Reference (source) asset path, e.g. /Root/Child/part.stl.",
);
const CANDIDATE_UUID: ArgSpec = string(
    "candidate_uuid",
    "--candidate-uuid",
    "Candidate (target) asset UUID.",
);
const CANDIDATE_PATH: ArgSpec = string(
    "candidate_path",
    "--candidate-path",
    "Candidate (target) asset path, e.g. /Root/Child/part.stl.",
);
const REFERENCE_AND_CANDIDATE: &[&[&str]] = &[
    &["reference_uuid", "reference_path"],
    &["candidate_uuid", "candidate_path"],
];

const THRESHOLD: ArgSpec = number(
    "threshold",
    "--threshold",
    "Similarity threshold (0.00 to 100.00). Default 80.0.",
)
.range(0.0, 100.0);
const SIZE_THRESHOLD: ArgSpec = number(
    "threshold",
    "--threshold",
    "Size threshold (0.00 to 100.00): filters matches by geometric size relative to the reference asset; higher is stricter, 0 disables size filtering. Default 80.0.",
)
.range(0.0, 100.0);
const EXCLUSIVE: ArgSpec = flag(
    "exclusive",
    "--exclusive",
    "Only show matches where both assets belong to the specified folder paths.",
);
const RECURSIVE: ArgSpec = flag(
    "recursive",
    "--recursive",
    "Include assets in subfolders, not just those directly in the folder.",
);
const CHECKPOINT: ArgSpec = string(
    "checkpoint",
    "--checkpoint",
    "Checkpoint file path. Each completed search is recorded there; re-running with the same file reuses recorded results and searches only the remaining assets. Removed once the report is written.",
);
const LIMIT_100: ArgSpec = integer(
    "limit",
    "--limit",
    "Maximum number of results to return. Default 100.",
);
const LIMIT_1000: ArgSpec = integer(
    "limit",
    "--limit",
    "Maximum number of results to return. Default 1000.",
);

const ENV_NAME: ArgSpec = string("name", "--name", "Environment name.");

// ---------------------------------------------------------------------------
// Tool table
// ---------------------------------------------------------------------------

pub static TOOLS: &[ToolSpec] = &[
    // -- general --------------------------------------------------------------
    tool(
        "pcli2",
        "Physna Command Line Interface v2 (PCLI2). Lists folders (`pcli2 folder list`) or assets (`pcli2 asset list`) depending on `resource`. Prefer the dedicated `pcli2_folder_list` and `pcli2_asset_list` tools; this tool is kept for backward compatibility. For `asset`, provide `folder_path` or `folder_uuid`.",
        &[],
        &[
            string("resource", "", "Resource to list. Defaults to folder.").values(&["folder", "asset"]),
            TENANT,
            METADATA,
            HEADERS,
            PRETTY,
            FORMAT_JCT,
            FOLDER_UUID,
            FOLDER_PATH,
            RELOAD,
            flag("recursive", "--recursive", "Recursively list assets in subfolders (asset only)."),
        ],
        &[],
    ),
    tool(
        "pcli2_version",
        "Prints the installed pcli2 version (`pcli2 --version`).",
        &["--version"],
        &[],
        &[],
    ),
    tool(
        "pcli2_doctor",
        "Checks the local pcli2 setup: binary, configuration, credentials, token, tenant, caches, API and auth-server connectivity, and update status (`pcli2 doctor`). Use this first when other tools fail unexpectedly.",
        &["doctor"],
        &[string("format", "--format", "Output format. Default text.").values(&["text", "json"])],
        &[],
    ),
    // -- tenant ---------------------------------------------------------------
    tool(
        "pcli2_tenant_list",
        "Lists all tenants visible to the current credentials (`pcli2 tenant list`).",
        &["tenant", "list"],
        &[FORMAT_JC, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_tenant_get",
        "Shows the active tenant (`pcli2 tenant get`).",
        &["tenant", "get"],
        &[FORMAT_JCT, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_tenant_state",
        "Shows asset counts by processing state for a tenant (`pcli2 tenant state`).",
        &["tenant", "state"],
        &[
            TENANT,
            string("type", "--type", "Only count assets in this state.").values(&[
                "indexing",
                "finished",
                "failed",
                "unsupported",
                "no-3d-data",
                "missing-dependencies",
            ]),
            FORMAT_JC,
            PRETTY,
            HEADERS,
        ],
        &[],
    ),
    tool(
        "pcli2_tenant_use",
        "Sets the active tenant by short name (`pcli2 tenant use --name <name>`). Subsequent commands run against this tenant unless they pass `tenant` explicitly.",
        &["tenant", "use"],
        &[
            string("name", "--name", "Tenant short name, as shown by pcli2_tenant_list."),
            local("tenant_name", "Alias for `name` (kept for backward compatibility)."),
            flag("refresh", "--refresh", "Force refresh of cached tenant data from the API."),
            FORMAT_JC,
            PRETTY,
            HEADERS,
        ],
        &[&["name", "tenant_name"]],
    ),
    tool(
        "pcli2_tenant_clear",
        "Clears the active tenant selection (`pcli2 tenant clear`).",
        &["tenant", "clear"],
        &[YES],
        &[],
    ),
    tool(
        "pcli2_tenant_metadata_list",
        "Lists every metadata field registered in the tenant with its data type (`pcli2 tenant metadata list`). CSV output uses the ASSET_PATH,NAME,VALUE,TYPE layout accepted by pcli2_asset_metadata_create_batch.",
        &["tenant", "metadata", "list"],
        &[TENANT, FORMAT_JCT, PRETTY, HEADERS],
        &[],
    ),
    // -- folder ---------------------------------------------------------------
    tool(
        "pcli2_folder_list",
        "Lists folders (`pcli2 folder list`). Without a folder the whole hierarchy is listed; `format=tree` renders it as a tree.",
        &["folder", "list"],
        &[TENANT, METADATA, HEADERS, PRETTY, FORMAT_JCT, FOLDER_UUID, FOLDER_PATH, RELOAD],
        &[],
    ),
    tool(
        "pcli2_folder_get",
        "Gets folder details such as id, path and asset/folder counts (`pcli2 folder get`).",
        &["folder", "get"],
        &[TENANT, FOLDER_UUID, FOLDER_PATH, METADATA, HEADERS, PRETTY, FORMAT_JCT],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_create",
        "Creates a new folder under a parent folder (`pcli2 folder create`).",
        &["folder", "create"],
        &[
            TENANT,
            string("name", "--name", "Name of the new folder.").required(),
            PARENT_FOLDER_UUID,
            PARENT_FOLDER_PATH,
            YES,
        ],
        &[PARENT_ONE_OF],
    ),
    tool(
        "pcli2_folder_delete",
        "Deletes a folder (`pcli2 folder delete`). A non-empty folder is refused unless `force` is set, which deletes every asset and subfolder in it. Use `dry_run` to preview.",
        &["folder", "delete"],
        &[
            TENANT,
            FOLDER_UUID,
            FOLDER_PATH,
            flag("force", "--force", "Delete the folder together with every asset and subfolder in it."),
            DRY_RUN,
            YES,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_rename",
        "Renames a folder (`pcli2 folder rename`).",
        &["folder", "rename"],
        &[
            TENANT,
            FOLDER_UUID,
            FOLDER_PATH,
            string("name", "--name", "New folder name.").required(),
            YES,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_move",
        "Moves a folder under a new parent folder (`pcli2 folder move`).",
        &["folder", "move"],
        &[TENANT, FOLDER_UUID, FOLDER_PATH, PARENT_FOLDER_UUID, PARENT_FOLDER_PATH, YES],
        &[&["folder_uuid", "folder_path"], PARENT_ONE_OF],
    ),
    tool(
        "pcli2_folder_resolve",
        "Resolves a folder path to its UUID (`pcli2 folder resolve`).",
        &["folder", "resolve"],
        &[
            TENANT,
            FOLDER_PATH.required(),
            flag("reload", "--reload", "Force refresh of the folder cache from the API before resolving."),
        ],
        &[],
    ),
    tool(
        "pcli2_folder_download",
        "Downloads every asset file in a folder to a local directory on the MCP server host (`pcli2 folder download`).",
        &["folder", "download"],
        &[
            TENANT,
            FOLDER_UUID,
            FOLDER_PATH,
            string("output", "--output", "Output directory path. Defaults to a directory named after the folder in the current directory."),
            PROGRESS,
            CONCURRENT.describe("Maximum number of concurrent downloads (1-10). Default 1."),
            CONTINUE_ON_ERROR,
            DELAY,
            flag("resume", "--resume", "Skip files that already exist in the destination directory."),
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_upload",
        "Uploads every file in a local directory on the MCP server host to a Physna folder (`pcli2 folder upload`).",
        &["folder", "upload"],
        &[
            TENANT,
            FOLDER_UUID,
            FOLDER_PATH,
            string("input", "--input", "Local directory containing the files to upload.").required(),
            flag("skip_existing", "--skip-existing", "Skip assets that already exist in the target folder instead of failing."),
            PROGRESS,
            CONCURRENT.describe("Maximum number of concurrent uploads (1-10). Default 1."),
            DELAY,
            CONTINUE_ON_ERROR,
            DRY_RUN,
            YES,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_thumbnail",
        "Downloads thumbnails for all assets in a folder to a local directory on the MCP server host (`pcli2 folder thumbnail`). For a single asset image use pcli2_asset_thumbnail.",
        &["folder", "thumbnail"],
        &[
            TENANT,
            FOLDER_UUID,
            FOLDER_PATH,
            string("output", "--output", "Output directory path. Defaults to a directory named after the folder in the current directory."),
            PROGRESS,
            CONCURRENT.describe("Maximum number of concurrent downloads (1-10). Default 1."),
            CONTINUE_ON_ERROR,
            DELAY,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_folder_dependencies",
        "Gets dependencies for all assembly assets in one or more folders (`pcli2 folder dependencies`).",
        &["folder", "dependencies"],
        &[TENANT, FOLDER_PATHS, HEADERS, METADATA, PRETTY, FORMAT_JCT, PROGRESS],
        &[],
    ),
    tool(
        "pcli2_folder_geometric_match",
        "Finds geometrically similar assets for every asset in one or more folders (`pcli2 folder geometric-match`). Rows are ordered by asset pair in json/csv; `format=xls` writes an Excel report to `output`.",
        &["folder", "geometric-match"],
        &[
            TENANT,
            FOLDER_PATHS,
            RECURSIVE,
            THRESHOLD,
            EXCLUSIVE,
            HEADERS,
            METADATA,
            PRETTY,
            string("format", "--format", "Output format. Default json.").values(&["json", "csv", "xls"]),
            string("output", "--output", "Output file path, used with format=xls. Default match_report.xlsx."),
            CONCURRENT,
            CHECKPOINT,
            PROGRESS,
        ],
        &[],
    ),
    tool(
        "pcli2_folder_part_match",
        "Finds part matches (part-in-assembly search) for every asset in one or more folders (`pcli2 folder part-match`).",
        &["folder", "part-match"],
        &[
            TENANT,
            FOLDER_PATHS,
            RECURSIVE,
            THRESHOLD,
            EXCLUSIVE,
            HEADERS,
            METADATA,
            PRETTY,
            FORMAT_JC,
            CONCURRENT,
            CHECKPOINT,
            PROGRESS,
        ],
        &[],
    ),
    tool(
        "pcli2_folder_visual_match",
        "Finds visually similar assets for every asset in one or more folders (`pcli2 folder visual-match`).",
        &["folder", "visual-match"],
        &[
            TENANT,
            FOLDER_PATHS,
            LIMIT_100,
            SIZE_THRESHOLD,
            RECURSIVE,
            EXCLUSIVE,
            HEADERS,
            METADATA,
            PRETTY,
            FORMAT_JC,
            CONCURRENT,
            CHECKPOINT,
            PROGRESS,
        ],
        &[],
    ),
    // -- auth -----------------------------------------------------------------
    tool(
        "pcli2_auth_login",
        "Logs in with OAuth2 client credentials for the active environment (`pcli2 auth login`). Both values are required because the MCP server cannot prompt interactively. Only pass credentials the user explicitly supplied.",
        &["auth", "login"],
        &[
            string("client_id", "--client-id", "OAuth2 client ID.").required(),
            string("client_secret", "--client-secret", "OAuth2 client secret.").required(),
        ],
        &[],
    ),
    tool(
        "pcli2_auth_logout",
        "Logs out and clears the session (`pcli2 auth logout`).",
        &["auth", "logout"],
        &[YES],
        &[],
    ),
    tool(
        "pcli2_auth_get",
        "Returns the current access token (`pcli2 auth get`).",
        &["auth", "get"],
        &[FORMAT_JC, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_auth_clear_token",
        "Clears the cached access token; the next command re-authenticates (`pcli2 auth clear-token`).",
        &["auth", "clear-token"],
        &[YES],
        &[],
    ),
    tool(
        "pcli2_auth_expiration",
        "Shows when the current access token expires (`pcli2 auth expiration`).",
        &["auth", "expiration"],
        &[],
        &[],
    ),
    // -- asset ----------------------------------------------------------------
    tool(
        "pcli2_asset_list",
        "Lists the assets in a folder (`pcli2 asset list`). Provide `folder_path` or `folder_uuid`.",
        &["asset", "list"],
        &[
            TENANT,
            FOLDER_PATH,
            FOLDER_UUID,
            flag("recursive", "--recursive", "Recursively list assets in subfolders."),
            RELOAD,
            METADATA,
            HEADERS,
            PRETTY,
            FORMAT_JC,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_get",
        "Gets asset details (`pcli2 asset get`).",
        &["asset", "get"],
        &[TENANT, UUID, PATH, HEADERS, METADATA, PRETTY, FORMAT_JC],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_create",
        "Uploads a local file on the MCP server host as a new asset in a folder (`pcli2 asset create`).",
        &["asset", "create"],
        &[
            TENANT,
            string("input", "--input", "Local file to upload.").required(),
            FOLDER_UUID,
            FOLDER_PATH,
            METADATA,
            HEADERS,
            PRETTY,
            FORMAT_JC,
            flag("override", "--override", "If the asset already exists, delete it and upload the new version in its place."),
            flag("restore_metadata", "--restore-metadata", "With `override`, preserve the existing asset's metadata and apply it to the new asset."),
            DRY_RUN,
            YES,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_create_batch",
        "Uploads multiple local files matching a glob pattern or comma-separated list as new assets in a folder (`pcli2 asset create-batch`).",
        &["asset", "create-batch"],
        &[
            TENANT,
            string("input", "--input", "Glob pattern or comma-separated list of files to upload, e.g. \"data/*.stl\" or \"a.stl,b.stl\".").required(),
            FOLDER_UUID,
            FOLDER_PATH,
            METADATA,
            HEADERS,
            PRETTY,
            FORMAT_JC,
            integer("concurrent", "--concurrent", "Maximum number of concurrent uploads. Default 5."),
            PROGRESS,
            flag("skip_existing", "--skip-existing", "Skip files whose name already exists in the target folder, so an interrupted batch can be re-run."),
            DRY_RUN,
            YES,
        ],
        FOLDER_UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_delete",
        "Deletes an asset (`pcli2 asset delete`). Use `dry_run` to preview.",
        &["asset", "delete"],
        &[TENANT, UUID, PATH, DRY_RUN, YES],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_download",
        "Downloads an asset's file to the MCP server host (`pcli2 asset download`).",
        &["asset", "download"],
        &[
            TENANT,
            UUID,
            PATH,
            string("output", "--output", "Output file path. Defaults to the asset filename in the current directory."),
        ],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_dependencies",
        "Gets the dependency tree of an assembly asset (`pcli2 asset dependencies`).",
        &["asset", "dependencies"],
        &[TENANT, UUID, PATH, METADATA, HEADERS, PRETTY, FORMAT_JCT],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_dependency_diff",
        "Diffs the dependency trees of two assets (`pcli2 asset dependency-diff`).",
        &["asset", "dependency-diff"],
        &[
            TENANT,
            REFERENCE_UUID,
            REFERENCE_PATH,
            CANDIDATE_UUID,
            CANDIDATE_PATH,
            METADATA,
            HEADERS,
            PRETTY,
            FORMAT_JCT,
        ],
        REFERENCE_AND_CANDIDATE,
    ),
    tool(
        "pcli2_asset_geometric_match",
        "Finds geometrically similar assets for a reference asset (`pcli2 asset geometric-match`).",
        &["asset", "geometric-match"],
        &[TENANT, UUID, PATH, THRESHOLD, HEADERS, METADATA, PRETTY, FORMAT_JC],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_geometric_match",
        "Alias of pcli2_asset_geometric_match, kept for backward compatibility (`pcli2 asset geometric-match`).",
        &["asset", "geometric-match"],
        &[TENANT, UUID, PATH, THRESHOLD, HEADERS, METADATA, PRETTY, FORMAT_JC],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_part_match",
        "Finds assemblies or parts matching a reference asset using the part search algorithm (`pcli2 asset part-match`).",
        &["asset", "part-match"],
        &[TENANT, UUID, PATH, THRESHOLD, HEADERS, METADATA, PRETTY, FORMAT_JC],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_visual_match",
        "Finds visually similar assets for a reference asset (`pcli2 asset visual-match`).",
        &["asset", "visual-match"],
        &[TENANT, UUID, PATH, LIMIT_100, SIZE_THRESHOLD, HEADERS, METADATA, PRETTY, FORMAT_JC],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_text_match",
        "Finds assets by text search over names and metadata (`pcli2 asset text-match`). Exact (quoted) search by default; set `fuzzy` for fuzzy matching.",
        &["asset", "text-match"],
        &[
            TENANT,
            string("text", "--text", "Text query to search for.").required(),
            flag("fuzzy", "--fuzzy", "Perform a fuzzy search instead of an exact search."),
            LIMIT_1000,
            HEADERS,
            METADATA,
            PRETTY,
            FORMAT_JC,
        ],
        &[],
    ),
    tool(
        "pcli2_asset_similarity",
        "Gets the pairwise match scores between two specific assets (`pcli2 asset similarity`).",
        &["asset", "similarity"],
        &[
            TENANT,
            REFERENCE_UUID,
            REFERENCE_PATH,
            CANDIDATE_UUID,
            CANDIDATE_PATH,
            HEADERS,
            METADATA,
            PRETTY,
            FORMAT_JC,
        ],
        REFERENCE_AND_CANDIDATE,
    ),
    tool(
        "pcli2_asset_reprocess",
        "Reprocesses an asset to refresh its analysis (`pcli2 asset reprocess`).",
        &["asset", "reprocess"],
        &[TENANT, UUID, PATH, YES],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_counts",
        "Shows an asset health report for the tenant: counts by processing state, by file type, and assemblies vs parts (`pcli2 asset counts`).",
        &["asset", "counts"],
        &[TENANT, METADATA, HEADERS, PRETTY, FORMAT_JC],
        &[],
    ),
    tool(
        "pcli2_asset_inventory",
        "Lists the complete inventory of every asset in the tenant (`pcli2 asset inventory`). Output can be very large.",
        &["asset", "inventory"],
        &[TENANT, METADATA, HEADERS, PRETTY, FORMAT_JC],
        &[],
    ),
    tool(
        "pcli2_asset_thumbnail",
        "Fetches an asset's thumbnail image (`pcli2 asset thumbnail`) and returns an HTML snippet embedding it. `response_mode` 'url' (default) returns a short HTTP URL served by this MCP server, which is efficient for LLM context; 'data_url' embeds the PNG as a base64 data URI, which is self-contained but costs tens of thousands of tokens.",
        &["asset", "thumbnail"],
        &[
            TENANT,
            UUID,
            PATH,
            local("response_mode", "How to return the image: 'url' (default, recommended) or 'data_url'.").values(&["url", "data_url"]),
        ],
        UUID_OR_PATH,
    ),
    // -- asset metadata ---------------------------------------------------------
    tool(
        "pcli2_asset_metadata_get",
        "Gets the metadata fields of an asset (`pcli2 asset metadata get`).",
        &["asset", "metadata", "get"],
        &[TENANT, UUID, PATH, FORMAT_JC, PRETTY, HEADERS, METADATA],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_metadata_create",
        "Adds or updates one metadata field on an asset (`pcli2 asset metadata create`).",
        &["asset", "metadata", "create"],
        &[
            TENANT,
            UUID,
            PATH,
            string("name", "--name", "Metadata field name.").required(),
            string("value", "--value", "Metadata field value.").required(),
            string("type", "--type", "Metadata field type. Default text.").values(&["text", "number", "boolean", "url"]),
            YES,
        ],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_metadata_delete",
        "Deletes specific metadata fields from an asset (`pcli2 asset metadata delete`).",
        &["asset", "metadata", "delete"],
        &[
            TENANT,
            UUID,
            PATH,
            string_list("name", "--name", "Metadata field name(s): a string, a comma-separated string, or an array of strings.").required(),
            FORMAT_JC,
            YES,
        ],
        UUID_OR_PATH,
    ),
    tool(
        "pcli2_asset_metadata_create_batch",
        "Creates or updates metadata for many assets from a CSV file on the MCP server host (`pcli2 asset metadata create-batch`). Supports the classic layout (ASSET_PATH,NAME,VALUE,TYPE rows) and the Physna UI export layout (one row per asset with metadata:<field> columns); the layout is auto-detected unless `csv_format` is set.",
        &["asset", "metadata", "create-batch"],
        &[
            TENANT,
            string("input", "--input", "CSV file with the metadata entries.").required(),
            string("csv_format", "--csv-format", "CSV layout. Default auto.").values(&["auto", "classic", "ui"]),
            PROGRESS,
            flag("delete_if_empty", "--delete-if-empty", "Delete a metadata field from the asset when the file has an empty value for it (by default empty values are skipped)."),
            CONTINUE_ON_ERROR,
            YES,
        ],
        &[],
    ),
    tool(
        "pcli2_asset_metadata_inference",
        "Copies metadata fields from a reference asset to geometrically similar assets (`pcli2 asset metadata inference`).",
        &["asset", "metadata", "inference"],
        &[
            TENANT,
            PATH.describe("Reference asset path, e.g. /Root/Folder/Asset.stl.").required(),
            string_list("name", "--name", "Metadata field name(s) to copy: a string, a comma-separated string, or an array of strings.").required(),
            THRESHOLD,
            flag("exclusive", "--exclusive", "Only apply to assets in the same parent folder as the reference asset."),
            FORMAT_JC,
            HEADERS,
            PRETTY,
            YES,
        ],
        &[],
    ),
    // -- config -----------------------------------------------------------------
    tool(
        "pcli2_config_get",
        "Shows the pcli2 configuration (`pcli2 config get`).",
        &["config", "get"],
        &[FORMAT_JCT, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_config_get_path",
        "Shows the path of the pcli2 configuration file (`pcli2 config get path`).",
        &["config", "get", "path"],
        &[FORMAT_JCT],
        &[],
    ),
    tool(
        "pcli2_config_validate",
        "Validates the pcli2 configuration and credentials (`pcli2 config validate`).",
        &["config", "validate"],
        &[flag("api", "--api", "Also test API connectivity (requires valid credentials).")],
        &[],
    ),
    tool(
        "pcli2_config_export",
        "Exports the pcli2 configuration to a file on the MCP server host (`pcli2 config export`).",
        &["config", "export"],
        &[string("output", "--output", "Output file path.")],
        &[],
    ),
    tool(
        "pcli2_config_import",
        "Imports pcli2 configuration from a file on the MCP server host (`pcli2 config import`).",
        &["config", "import"],
        &[string("input", "--input", "Configuration file to import."), YES],
        &[],
    ),
    // -- environment ------------------------------------------------------------
    tool(
        "pcli2_environment_list",
        "Lists all configured environments (`pcli2 env list`).",
        &["env", "list"],
        &[FORMAT_JC, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_environment_get",
        "Gets environment details (`pcli2 env get`). Defaults to the active environment.",
        &["env", "get"],
        &[ENV_NAME.describe("Environment name. Defaults to the active environment."), FORMAT_JC, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_environment_use",
        "Switches the active environment (`pcli2 env use --name <name>`).",
        &["env", "use"],
        &[ENV_NAME.describe("Name of the environment to switch to.").required()],
        &[],
    ),
    tool(
        "pcli2_environment_add",
        "Adds a new environment configuration (`pcli2 env add`).",
        &["env", "add"],
        &[
            ENV_NAME.describe("Name of the new environment.").required(),
            string("api_url", "--api-url", "API base URL, e.g. https://app-api.physna.com/v3."),
            string("ui_url", "--ui-url", "UI base URL, e.g. https://app.physna.com."),
            string("auth_url", "--auth-url", "Authentication URL, e.g. https://physna-app.auth.us-east-2.amazoncognito.com/oauth2/token."),
        ],
        &[],
    ),
    tool(
        "pcli2_environment_remove",
        "Removes an environment configuration (`pcli2 env remove --name <name>`).",
        &["env", "remove"],
        &[ENV_NAME.describe("Name of the environment to remove.").required(), YES],
        &[],
    ),
    tool(
        "pcli2_environment_reset",
        "Resets all environment configurations to a blank state (`pcli2 env reset`).",
        &["env", "reset"],
        &[YES],
        &[],
    ),
    // -- user -------------------------------------------------------------------
    tool(
        "pcli2_user_list",
        "Lists users in the current tenant (`pcli2 user list`).",
        &["user", "list"],
        &[FORMAT_JCT, PRETTY, HEADERS],
        &[],
    ),
    tool(
        "pcli2_user_get",
        "Gets details for a specific user (`pcli2 user get <user_id>`).",
        &["user", "get"],
        &[
            positional("user_id", "The ID of the user to retrieve.").required(),
            FORMAT_JCT,
            PRETTY,
            HEADERS,
        ],
        &[],
    ),
    // -- cache ------------------------------------------------------------------
    tool(
        "pcli2_cache_clear",
        "Clears pcli2's local caches: folder hierarchy, metadata and tenants (`pcli2 cache clear`). Set one of the flags to clear only that cache.",
        &["cache", "clear"],
        &[
            flag("folder", "--folder", "Clear only the folder cache."),
            flag("metadata", "--metadata", "Clear only the metadata cache."),
            flag("tenant", "--tenant", "Clear only the tenant cache."),
            YES,
        ],
        &[],
    ),
    // -- MCP server local -------------------------------------------------------
    tool(
        "pcli2_thumbnail_cache_cleanup",
        "Removes expired thumbnails from this MCP server's local thumbnail cache to free disk space.",
        &[],
        &[],
        &[],
    ),
];

pub fn find_tool(name: &str) -> Option<&'static ToolSpec> {
    TOOLS.iter().find(|spec| spec.name == name)
}

// ---------------------------------------------------------------------------
// JSON schema generation
// ---------------------------------------------------------------------------

pub fn schema(spec: &ToolSpec) -> Value {
    let mut properties = Map::new();
    for arg in spec.args {
        let mut prop = Map::new();
        match arg.kind {
            Kind::Flag => {
                prop.insert("type".into(), json!("boolean"));
            }
            Kind::Str | Kind::Positional | Kind::Local => {
                prop.insert("type".into(), json!("string"));
            }
            Kind::Num => {
                prop.insert("type".into(), json!("number"));
            }
            Kind::Int => {
                prop.insert("type".into(), json!("integer"));
            }
            Kind::StrList => {
                prop.insert(
                    "oneOf".into(),
                    json!([
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } }
                    ]),
                );
            }
        }
        if !arg.enum_values.is_empty() {
            prop.insert("enum".into(), json!(arg.enum_values));
        }
        if let Some((min, max)) = arg.range {
            prop.insert("minimum".into(), json!(min));
            prop.insert("maximum".into(), json!(max));
        }
        prop.insert("description".into(), json!(arg.description));
        properties.insert(arg.key.to_string(), Value::Object(prop));
    }

    let required: Vec<&str> = spec
        .args
        .iter()
        .filter(|arg| arg.required)
        .map(|arg| arg.key)
        .collect();

    let mut description = spec.description.to_string();
    for group in spec.one_of {
        description.push_str(&format!(" Requires one of: {}.", quote_keys(group)));
    }

    json!({
        "name": spec.name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required
        }
    })
}

fn quote_keys(keys: &[&str]) -> String {
    keys.iter()
        .map(|key| format!("'{}'", key))
        .collect::<Vec<_>>()
        .join(" or ")
}

// ---------------------------------------------------------------------------
// Argument construction
// ---------------------------------------------------------------------------

/// Whether an argument carries a usable value (null, empty strings and empty
/// arrays count as absent).
fn is_present(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

/// Builds the pcli2 argv (without the executable) for a tool invocation.
pub fn build_args(spec: &ToolSpec, args: &Value) -> Result<Vec<String>, String> {
    let empty = Map::new();
    let obj = match args {
        Value::Object(obj) => obj,
        Value::Null => &empty,
        _ => return Err("Tool arguments must be a JSON object".to_string()),
    };

    for arg in spec.args {
        if arg.required && !is_present(obj.get(arg.key)) {
            return Err(format!("Missing required argument: '{}'", arg.key));
        }
    }
    for group in spec.one_of {
        if !group.iter().any(|key| is_present(obj.get(*key))) {
            return Err(format!(
                "Missing required argument: provide one of {}",
                quote_keys(group)
            ));
        }
    }

    let mut out: Vec<String> = spec.command.iter().map(|s| s.to_string()).collect();
    for arg in spec.args {
        let value = obj.get(arg.key);
        if !is_present(value) {
            continue;
        }
        let value = value.expect("present");
        match arg.kind {
            Kind::Local => {
                let text = parse_string(arg.key, value)?;
                check_enum(arg, &text)?;
            }
            Kind::Flag => {
                if parse_bool(arg.key, value)? {
                    out.push(arg.flag.to_string());
                }
            }
            Kind::Str => {
                let text = parse_string(arg.key, value)?;
                check_enum(arg, &text)?;
                out.push(arg.flag.to_string());
                out.push(text);
            }
            Kind::Positional => {
                let text = parse_string(arg.key, value)?;
                check_enum(arg, &text)?;
                out.push(text);
            }
            Kind::Num => {
                let number = parse_f64(arg.key, value)?;
                check_range(arg, number)?;
                out.push(arg.flag.to_string());
                out.push(number.to_string());
            }
            Kind::Int => {
                let number = parse_u64(arg.key, value)?;
                check_range(arg, number as f64)?;
                out.push(arg.flag.to_string());
                out.push(number.to_string());
            }
            Kind::StrList => {
                for item in parse_string_list(arg.key, value)? {
                    check_enum(arg, &item)?;
                    out.push(arg.flag.to_string());
                    out.push(item);
                }
            }
        }
    }
    Ok(out)
}

fn check_enum(arg: &ArgSpec, text: &str) -> Result<(), String> {
    if !arg.enum_values.is_empty() && !arg.enum_values.contains(&text) {
        return Err(format!(
            "Invalid argument '{}': '{}' is not one of {}",
            arg.key,
            text,
            arg.enum_values.join(", ")
        ));
    }
    Ok(())
}

fn check_range(arg: &ArgSpec, number: f64) -> Result<(), String> {
    if let Some((min, max)) = arg.range
        && (number < min || number > max)
    {
        return Err(format!(
            "Invalid argument '{}': value {} must be between {} and {}",
            arg.key, number, min, max
        ));
    }
    Ok(())
}

fn parse_bool(key: &str, value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(b) => Ok(*b),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(true),
            "false" | "no" | "0" => Ok(false),
            _ => Err(format!("Invalid argument '{}': expected a boolean", key)),
        },
        Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Err(format!("Invalid argument '{}': expected a boolean", key)),
    }
}

fn parse_string(key: &str, value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(format!("Invalid argument '{}': expected a string", key)),
    }
}

fn parse_f64(key: &str, value: &Value) -> Result<f64, String> {
    let parsed = match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    parsed
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("Invalid argument '{}': expected a number", key))
}

fn parse_u64(key: &str, value: &Value) -> Result<u64, String> {
    let parsed = match value {
        Value::Number(n) => n.as_u64().or_else(|| {
            n.as_f64()
                .filter(|f| f.fract() == 0.0 && *f >= 0.0)
                .map(|f| f as u64)
        }),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    };
    parsed.ok_or_else(|| {
        format!(
            "Invalid argument '{}': expected a non-negative integer",
            key
        )
    })
}

fn parse_string_list(key: &str, value: &Value) -> Result<Vec<String>, String> {
    let items: Vec<String> = match value {
        Value::Array(values) => values
            .iter()
            .map(|v| parse_string(key, v))
            .collect::<Result<Vec<_>, _>>()?,
        other => vec![parse_string(key, other)?],
    };
    let items: Vec<String> = items
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return Err(format!("Missing required argument: '{}'", key));
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn argv(name: &str, args: Value) -> Result<Vec<String>, String> {
        let spec = find_tool(name).unwrap_or_else(|| panic!("tool {} not found", name));
        build_args(spec, &args)
    }

    fn ok(name: &str, args: Value) -> Vec<String> {
        argv(name, args).unwrap_or_else(|err| panic!("{}: {}", name, err))
    }

    fn err(name: &str, args: Value) -> String {
        match argv(name, args) {
            Ok(v) => panic!("{}: expected error, got {:?}", name, v),
            Err(e) => e,
        }
    }

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// One "happy path" case per tool with every argument set, plus the
    /// expected argv. `test_every_tool_has_a_case` fails when a tool is
    /// added to TOOLS without a case here.
    fn cases() -> Vec<(&'static str, Value, Vec<String>)> {
        vec![
            (
                "pcli2",
                json!({"resource": "asset", "folder_path": "/A", "recursive": true, "format": "csv"}),
                // The legacy tool delegates to pcli2_asset_list / pcli2_folder_list.
                v(&[
                    "asset",
                    "list",
                    "--folder-path",
                    "/A",
                    "--recursive",
                    "--format",
                    "csv",
                ]),
            ),
            ("pcli2_version", json!({}), v(&["--version"])),
            (
                "pcli2_doctor",
                json!({"format": "json"}),
                v(&["doctor", "--format", "json"]),
            ),
            (
                "pcli2_tenant_list",
                json!({"format": "csv", "pretty": true, "headers": true}),
                v(&["tenant", "list", "--format", "csv", "--pretty", "--headers"]),
            ),
            (
                "pcli2_tenant_get",
                json!({"format": "tree"}),
                v(&["tenant", "get", "--format", "tree"]),
            ),
            (
                "pcli2_tenant_state",
                json!({"tenant": "demo", "type": "failed", "format": "csv", "headers": true}),
                v(&[
                    "tenant",
                    "state",
                    "--tenant",
                    "demo",
                    "--type",
                    "failed",
                    "--format",
                    "csv",
                    "--headers",
                ]),
            ),
            (
                "pcli2_tenant_use",
                json!({"name": "demo-1", "refresh": true, "format": "json"}),
                v(&[
                    "tenant",
                    "use",
                    "--name",
                    "demo-1",
                    "--refresh",
                    "--format",
                    "json",
                ]),
            ),
            (
                "pcli2_tenant_clear",
                json!({"yes": true}),
                v(&["tenant", "clear", "--yes"]),
            ),
            (
                "pcli2_tenant_metadata_list",
                json!({"tenant": "demo", "format": "csv", "headers": true}),
                v(&[
                    "tenant",
                    "metadata",
                    "list",
                    "--tenant",
                    "demo",
                    "--format",
                    "csv",
                    "--headers",
                ]),
            ),
            (
                "pcli2_folder_list",
                json!({"tenant": "t", "metadata": true, "headers": true, "pretty": true, "format": "tree", "folder_uuid": "u", "folder_path": "/p", "reload": true}),
                v(&[
                    "folder",
                    "list",
                    "--tenant",
                    "t",
                    "--metadata",
                    "--headers",
                    "--pretty",
                    "--format",
                    "tree",
                    "--folder-uuid",
                    "u",
                    "--folder-path",
                    "/p",
                    "--reload",
                ]),
            ),
            (
                "pcli2_folder_get",
                json!({"folder_path": "/A/B", "metadata": true, "format": "json"}),
                v(&[
                    "folder",
                    "get",
                    "--folder-path",
                    "/A/B",
                    "--metadata",
                    "--format",
                    "json",
                ]),
            ),
            (
                "pcli2_folder_create",
                json!({"name": "New", "parent_folder_path": "/A", "yes": true}),
                v(&[
                    "folder",
                    "create",
                    "--name",
                    "New",
                    "--parent-folder-path",
                    "/A",
                    "--yes",
                ]),
            ),
            (
                "pcli2_folder_delete",
                json!({"folder_uuid": "u1", "force": true, "dry_run": true, "yes": true}),
                v(&[
                    "folder",
                    "delete",
                    "--folder-uuid",
                    "u1",
                    "--force",
                    "--dry-run",
                    "--yes",
                ]),
            ),
            (
                "pcli2_folder_rename",
                json!({"folder_path": "/A/Old", "name": "New"}),
                v(&[
                    "folder",
                    "rename",
                    "--folder-path",
                    "/A/Old",
                    "--name",
                    "New",
                ]),
            ),
            (
                "pcli2_folder_move",
                json!({"folder_path": "/A/X", "parent_folder_uuid": "pu"}),
                v(&[
                    "folder",
                    "move",
                    "--folder-path",
                    "/A/X",
                    "--parent-folder-uuid",
                    "pu",
                ]),
            ),
            (
                "pcli2_folder_resolve",
                json!({"folder_path": "/A", "reload": true}),
                v(&["folder", "resolve", "--folder-path", "/A", "--reload"]),
            ),
            (
                "pcli2_folder_download",
                json!({"folder_path": "/A", "output": "./out", "progress": true, "concurrent": 4, "continue_on_error": true, "delay": 2, "resume": true}),
                v(&[
                    "folder",
                    "download",
                    "--folder-path",
                    "/A",
                    "--output",
                    "./out",
                    "--progress",
                    "--concurrent",
                    "4",
                    "--continue-on-error",
                    "--delay",
                    "2",
                    "--resume",
                ]),
            ),
            (
                "pcli2_folder_upload",
                json!({"folder_path": "/A", "input": "./in", "skip_existing": true, "progress": true, "concurrent": 2, "delay": 1, "continue_on_error": true, "dry_run": true}),
                v(&[
                    "folder",
                    "upload",
                    "--folder-path",
                    "/A",
                    "--input",
                    "./in",
                    "--skip-existing",
                    "--progress",
                    "--concurrent",
                    "2",
                    "--delay",
                    "1",
                    "--continue-on-error",
                    "--dry-run",
                ]),
            ),
            (
                "pcli2_folder_thumbnail",
                json!({"folder_uuid": "fu", "output": "./thumbs", "progress": true, "concurrent": 3, "continue_on_error": true, "delay": 0}),
                v(&[
                    "folder",
                    "thumbnail",
                    "--folder-uuid",
                    "fu",
                    "--output",
                    "./thumbs",
                    "--progress",
                    "--concurrent",
                    "3",
                    "--continue-on-error",
                    "--delay",
                    "0",
                ]),
            ),
            (
                "pcli2_folder_dependencies",
                json!({"folder_path": ["/A", "/B"], "headers": true, "format": "tree", "progress": true}),
                v(&[
                    "folder",
                    "dependencies",
                    "--folder-path",
                    "/A",
                    "--folder-path",
                    "/B",
                    "--headers",
                    "--format",
                    "tree",
                    "--progress",
                ]),
            ),
            (
                "pcli2_folder_geometric_match",
                json!({"folder_path": "/A", "recursive": true, "threshold": 85, "exclusive": true, "format": "xls", "output": "r.xlsx", "concurrent": 5, "checkpoint": "cp.json", "progress": true}),
                v(&[
                    "folder",
                    "geometric-match",
                    "--folder-path",
                    "/A",
                    "--recursive",
                    "--threshold",
                    "85",
                    "--exclusive",
                    "--format",
                    "xls",
                    "--output",
                    "r.xlsx",
                    "--concurrent",
                    "5",
                    "--checkpoint",
                    "cp.json",
                    "--progress",
                ]),
            ),
            (
                "pcli2_folder_part_match",
                json!({"folder_path": "/A,/B", "threshold": 70.5, "format": "csv", "checkpoint": "cp"}),
                v(&[
                    "folder",
                    "part-match",
                    "--folder-path",
                    "/A,/B",
                    "--threshold",
                    "70.5",
                    "--format",
                    "csv",
                    "--checkpoint",
                    "cp",
                ]),
            ),
            (
                "pcli2_folder_visual_match",
                json!({"folder_path": "/A", "limit": 20, "threshold": 0, "recursive": true, "exclusive": true, "format": "json", "concurrent": 1}),
                v(&[
                    "folder",
                    "visual-match",
                    "--folder-path",
                    "/A",
                    "--limit",
                    "20",
                    "--threshold",
                    "0",
                    "--recursive",
                    "--exclusive",
                    "--format",
                    "json",
                    "--concurrent",
                    "1",
                ]),
            ),
            (
                "pcli2_auth_login",
                json!({"client_id": "id", "client_secret": "sec"}),
                v(&[
                    "auth",
                    "login",
                    "--client-id",
                    "id",
                    "--client-secret",
                    "sec",
                ]),
            ),
            ("pcli2_auth_logout", json!({}), v(&["auth", "logout"])),
            (
                "pcli2_auth_get",
                json!({"pretty": true}),
                v(&["auth", "get", "--pretty"]),
            ),
            (
                "pcli2_auth_clear_token",
                json!({"yes": true}),
                v(&["auth", "clear-token", "--yes"]),
            ),
            (
                "pcli2_auth_expiration",
                json!({}),
                v(&["auth", "expiration"]),
            ),
            (
                "pcli2_asset_list",
                json!({"folder_uuid": "fu", "recursive": true, "reload": true, "metadata": true, "headers": true, "format": "csv"}),
                v(&[
                    "asset",
                    "list",
                    "--folder-uuid",
                    "fu",
                    "--recursive",
                    "--reload",
                    "--metadata",
                    "--headers",
                    "--format",
                    "csv",
                ]),
            ),
            (
                "pcli2_asset_get",
                json!({"tenant": "t", "uuid": "u", "headers": true, "metadata": true, "pretty": true, "format": "json"}),
                v(&[
                    "asset",
                    "get",
                    "--tenant",
                    "t",
                    "--uuid",
                    "u",
                    "--headers",
                    "--metadata",
                    "--pretty",
                    "--format",
                    "json",
                ]),
            ),
            (
                "pcli2_asset_create",
                json!({"input": "part.stl", "folder_path": "/A", "override": true, "restore_metadata": true, "dry_run": true, "format": "json"}),
                v(&[
                    "asset",
                    "create",
                    "--input",
                    "part.stl",
                    "--folder-path",
                    "/A",
                    "--format",
                    "json",
                    "--override",
                    "--restore-metadata",
                    "--dry-run",
                ]),
            ),
            (
                "pcli2_asset_create_batch",
                json!({"input": "data/*.stl", "folder_path": "/A", "concurrent": 8, "progress": true, "skip_existing": true}),
                v(&[
                    "asset",
                    "create-batch",
                    "--input",
                    "data/*.stl",
                    "--folder-path",
                    "/A",
                    "--concurrent",
                    "8",
                    "--progress",
                    "--skip-existing",
                ]),
            ),
            (
                "pcli2_asset_delete",
                json!({"path": "/A/p.stl", "dry_run": true, "yes": true}),
                v(&[
                    "asset",
                    "delete",
                    "--path",
                    "/A/p.stl",
                    "--dry-run",
                    "--yes",
                ]),
            ),
            (
                "pcli2_asset_download",
                json!({"uuid": "u", "output": "./p.stl"}),
                v(&["asset", "download", "--uuid", "u", "--output", "./p.stl"]),
            ),
            (
                "pcli2_asset_dependencies",
                json!({"path": "/A/asm.step", "format": "tree"}),
                v(&[
                    "asset",
                    "dependencies",
                    "--path",
                    "/A/asm.step",
                    "--format",
                    "tree",
                ]),
            ),
            (
                "pcli2_asset_dependency_diff",
                json!({"reference_path": "/A/a.step", "candidate_uuid": "cu", "format": "tree"}),
                v(&[
                    "asset",
                    "dependency-diff",
                    "--reference-path",
                    "/A/a.step",
                    "--candidate-uuid",
                    "cu",
                    "--format",
                    "tree",
                ]),
            ),
            (
                "pcli2_asset_geometric_match",
                json!({"path": "/A/p.stl", "threshold": 90, "format": "csv", "headers": true}),
                v(&[
                    "asset",
                    "geometric-match",
                    "--path",
                    "/A/p.stl",
                    "--threshold",
                    "90",
                    "--headers",
                    "--format",
                    "csv",
                ]),
            ),
            (
                "pcli2_geometric_match",
                json!({"uuid": "u", "threshold": 80.5}),
                v(&[
                    "asset",
                    "geometric-match",
                    "--uuid",
                    "u",
                    "--threshold",
                    "80.5",
                ]),
            ),
            (
                "pcli2_asset_part_match",
                json!({"uuid": "u", "threshold": 75, "metadata": true}),
                v(&[
                    "asset",
                    "part-match",
                    "--uuid",
                    "u",
                    "--threshold",
                    "75",
                    "--metadata",
                ]),
            ),
            (
                "pcli2_asset_visual_match",
                json!({"path": "/A/p.stl", "limit": 5, "threshold": 50, "pretty": true}),
                v(&[
                    "asset",
                    "visual-match",
                    "--path",
                    "/A/p.stl",
                    "--limit",
                    "5",
                    "--threshold",
                    "50",
                    "--pretty",
                ]),
            ),
            (
                "pcli2_asset_text_match",
                json!({"text": "bolt", "fuzzy": true, "limit": 10, "format": "csv"}),
                v(&[
                    "asset",
                    "text-match",
                    "--text",
                    "bolt",
                    "--fuzzy",
                    "--limit",
                    "10",
                    "--format",
                    "csv",
                ]),
            ),
            (
                "pcli2_asset_similarity",
                json!({"reference_uuid": "ru", "candidate_path": "/A/c.stl", "pretty": true}),
                v(&[
                    "asset",
                    "similarity",
                    "--reference-uuid",
                    "ru",
                    "--candidate-path",
                    "/A/c.stl",
                    "--pretty",
                ]),
            ),
            (
                "pcli2_asset_reprocess",
                json!({"path": "/A/p.stl"}),
                v(&["asset", "reprocess", "--path", "/A/p.stl"]),
            ),
            (
                "pcli2_asset_counts",
                json!({"tenant": "t", "pretty": true}),
                v(&["asset", "counts", "--tenant", "t", "--pretty"]),
            ),
            (
                "pcli2_asset_inventory",
                json!({"format": "csv", "headers": true, "metadata": true}),
                v(&[
                    "asset",
                    "inventory",
                    "--metadata",
                    "--headers",
                    "--format",
                    "csv",
                ]),
            ),
            (
                "pcli2_asset_thumbnail",
                json!({"path": "/A/p.stl", "response_mode": "data_url"}),
                v(&["asset", "thumbnail", "--path", "/A/p.stl"]),
            ),
            (
                "pcli2_asset_metadata_get",
                json!({"uuid": "u", "format": "csv", "headers": true}),
                v(&[
                    "asset",
                    "metadata",
                    "get",
                    "--uuid",
                    "u",
                    "--format",
                    "csv",
                    "--headers",
                ]),
            ),
            (
                "pcli2_asset_metadata_create",
                json!({"path": "/A/p.stl", "name": "Material", "value": "Steel", "type": "url"}),
                v(&[
                    "asset", "metadata", "create", "--path", "/A/p.stl", "--name", "Material",
                    "--value", "Steel", "--type", "url",
                ]),
            ),
            (
                "pcli2_asset_metadata_delete",
                json!({"path": "/A/p.stl", "name": ["Material", "Color"], "format": "json"}),
                v(&[
                    "asset", "metadata", "delete", "--path", "/A/p.stl", "--name", "Material",
                    "--name", "Color", "--format", "json",
                ]),
            ),
            (
                "pcli2_asset_metadata_create_batch",
                json!({"input": "meta.csv", "csv_format": "ui", "progress": true, "delete_if_empty": true, "continue_on_error": true}),
                v(&[
                    "asset",
                    "metadata",
                    "create-batch",
                    "--input",
                    "meta.csv",
                    "--csv-format",
                    "ui",
                    "--progress",
                    "--delete-if-empty",
                    "--continue-on-error",
                ]),
            ),
            (
                "pcli2_asset_metadata_inference",
                json!({"path": "/A/p.stl", "name": "Material,Color", "threshold": 95, "exclusive": true, "format": "csv"}),
                v(&[
                    "asset",
                    "metadata",
                    "inference",
                    "--path",
                    "/A/p.stl",
                    "--name",
                    "Material,Color",
                    "--threshold",
                    "95",
                    "--exclusive",
                    "--format",
                    "csv",
                ]),
            ),
            (
                "pcli2_config_get",
                json!({"format": "tree"}),
                v(&["config", "get", "--format", "tree"]),
            ),
            (
                "pcli2_config_get_path",
                json!({}),
                v(&["config", "get", "path"]),
            ),
            (
                "pcli2_config_validate",
                json!({"api": true}),
                v(&["config", "validate", "--api"]),
            ),
            (
                "pcli2_config_export",
                json!({"output": "cfg.yml"}),
                v(&["config", "export", "--output", "cfg.yml"]),
            ),
            (
                "pcli2_config_import",
                json!({"input": "cfg.yml"}),
                v(&["config", "import", "--input", "cfg.yml"]),
            ),
            (
                "pcli2_environment_list",
                json!({"headers": true}),
                v(&["env", "list", "--headers"]),
            ),
            (
                "pcli2_environment_get",
                json!({"name": "shared"}),
                v(&["env", "get", "--name", "shared"]),
            ),
            (
                "pcli2_environment_use",
                json!({"name": "shared"}),
                v(&["env", "use", "--name", "shared"]),
            ),
            (
                "pcli2_environment_add",
                json!({"name": "e", "api_url": "https://api", "ui_url": "https://ui", "auth_url": "https://auth"}),
                v(&[
                    "env",
                    "add",
                    "--name",
                    "e",
                    "--api-url",
                    "https://api",
                    "--ui-url",
                    "https://ui",
                    "--auth-url",
                    "https://auth",
                ]),
            ),
            (
                "pcli2_environment_remove",
                json!({"name": "e", "yes": true}),
                v(&["env", "remove", "--name", "e", "--yes"]),
            ),
            (
                "pcli2_environment_reset",
                json!({"yes": true}),
                v(&["env", "reset", "--yes"]),
            ),
            (
                "pcli2_user_list",
                json!({"format": "csv"}),
                v(&["user", "list", "--format", "csv"]),
            ),
            (
                "pcli2_user_get",
                json!({"user_id": "42", "pretty": true}),
                v(&["user", "get", "42", "--pretty"]),
            ),
            (
                "pcli2_cache_clear",
                json!({"folder": true, "metadata": true, "tenant": true}),
                v(&["cache", "clear", "--folder", "--metadata", "--tenant"]),
            ),
            ("pcli2_thumbnail_cache_cleanup", json!({}), v(&[])),
        ]
    }

    #[test]
    fn test_every_tool_has_a_case() {
        let covered: HashSet<&str> = cases().into_iter().map(|(name, _, _)| name).collect();
        let missing: Vec<&str> = TOOLS
            .iter()
            .map(|spec| spec.name)
            .filter(|name| !covered.contains(name))
            .collect();
        assert!(
            missing.is_empty(),
            "tools without an argv test case: {:?}",
            missing
        );
    }

    #[test]
    fn test_argv_for_every_tool() {
        for (name, args, expected) in cases() {
            let spec = find_tool(name).unwrap();
            // The legacy combined tool delegates to the asset/folder list specs.
            let spec = if name == "pcli2" {
                find_tool("pcli2_asset_list").unwrap()
            } else {
                spec
            };
            let actual = build_args(spec, &args).unwrap_or_else(|e| panic!("{}: {}", name, e));
            assert_eq!(actual, expected, "argv mismatch for {}", name);
        }
    }

    #[test]
    fn test_tool_names_unique_and_prefixed() {
        let mut seen = HashSet::new();
        for spec in TOOLS {
            assert!(spec.name.starts_with("pcli2"), "{}", spec.name);
            assert!(seen.insert(spec.name), "duplicate tool {}", spec.name);
        }
    }

    #[test]
    fn test_arg_keys_unique_per_tool() {
        for spec in TOOLS {
            let mut seen = HashSet::new();
            for arg in spec.args {
                assert!(
                    seen.insert(arg.key),
                    "{} has duplicate arg {}",
                    spec.name,
                    arg.key
                );
            }
            for group in spec.one_of {
                for key in *group {
                    assert!(
                        seen.contains(key),
                        "{} one_of references unknown arg {}",
                        spec.name,
                        key
                    );
                }
            }
        }
    }

    #[test]
    fn test_schema_shape() {
        for spec in TOOLS {
            let s = schema(spec);
            assert_eq!(s["name"], spec.name);
            assert!(s["description"].as_str().unwrap().len() > 10);
            let props = s["inputSchema"]["properties"].as_object().unwrap();
            assert_eq!(props.len(), spec.args.len());
            for key in s["inputSchema"]["required"].as_array().unwrap() {
                assert!(props.contains_key(key.as_str().unwrap()));
            }
            for group in spec.one_of {
                assert!(
                    s["description"]
                        .as_str()
                        .unwrap()
                        .contains(&quote_keys(group))
                );
            }
        }
    }

    #[test]
    fn test_schema_details() {
        let s = schema(find_tool("pcli2_folder_geometric_match").unwrap());
        let props = &s["inputSchema"]["properties"];
        assert_eq!(props["threshold"]["type"], "number");
        assert_eq!(props["threshold"]["minimum"], 0.0);
        assert_eq!(props["threshold"]["maximum"], 100.0);
        assert_eq!(props["concurrent"]["type"], "integer");
        assert_eq!(props["format"]["enum"], json!(["json", "csv", "xls"]));
        assert!(props["folder_path"]["oneOf"].is_array());
        assert_eq!(props["recursive"]["type"], "boolean");
        assert_eq!(s["inputSchema"]["required"], json!(["folder_path"]));

        let s = schema(find_tool("pcli2_user_get").unwrap());
        assert_eq!(s["inputSchema"]["properties"]["user_id"]["type"], "string");
        assert_eq!(s["inputSchema"]["required"], json!(["user_id"]));

        let s = schema(find_tool("pcli2_asset_thumbnail").unwrap());
        assert_eq!(
            s["inputSchema"]["properties"]["response_mode"]["enum"],
            json!(["url", "data_url"])
        );
    }

    #[test]
    fn test_missing_required() {
        assert!(err("pcli2_asset_text_match", json!({})).contains("'text'"));
        assert!(err("pcli2_asset_text_match", json!({"text": "  "})).contains("'text'"));
        assert!(
            err("pcli2_folder_dependencies", json!({"folder_path": []})).contains("'folder_path'")
        );
        assert!(err("pcli2_user_get", json!({})).contains("'user_id'"));
        assert!(err("pcli2_environment_use", json!({})).contains("'name'"));
        assert!(
            err(
                "pcli2_asset_metadata_create",
                json!({"path": "/a", "name": "n"})
            )
            .contains("'value'")
        );
    }

    #[test]
    fn test_missing_one_of() {
        assert!(err("pcli2_asset_get", json!({})).contains("provide one of 'uuid' or 'path'"));
        assert!(
            err("pcli2_folder_get", json!({"format": "json"}))
                .contains("'folder_uuid' or 'folder_path'")
        );
        assert!(err("pcli2_asset_list", json!({})).contains("'folder_uuid' or 'folder_path'"));
        let e = err("pcli2_asset_similarity", json!({"reference_uuid": "r"}));
        assert!(e.contains("'candidate_uuid' or 'candidate_path'"), "{}", e);
        let e = err("pcli2_folder_move", json!({"folder_path": "/a"}));
        assert!(
            e.contains("'parent_folder_uuid' or 'parent_folder_path'"),
            "{}",
            e
        );
        assert!(err("pcli2_tenant_use", json!({})).contains("'name' or 'tenant_name'"));
        // null values count as absent
        assert!(err("pcli2_asset_get", json!({"uuid": null})).contains("provide one of"));
    }

    #[test]
    fn test_range_validation() {
        assert!(
            err(
                "pcli2_asset_geometric_match",
                json!({"uuid": "u", "threshold": 100.5})
            )
            .contains("between 0 and 100")
        );
        assert!(
            err(
                "pcli2_asset_geometric_match",
                json!({"uuid": "u", "threshold": -1})
            )
            .contains("between 0 and 100")
        );
        assert!(
            err(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": 11})
            )
            .contains("between 1 and 10")
        );
        assert!(
            err(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": 0})
            )
            .contains("between 1 and 10")
        );
        assert!(
            err(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "delay": 181})
            )
            .contains("between 0 and 180")
        );
        assert!(
            err(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": -3})
            )
            .contains("non-negative integer")
        );
        assert!(
            err(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": 2.5})
            )
            .contains("non-negative integer")
        );
    }

    #[test]
    fn test_enum_validation() {
        assert!(
            err("pcli2_tenant_list", json!({"format": "xml"})).contains("not one of json, csv")
        );
        assert!(err("pcli2_tenant_state", json!({"type": "bogus"})).contains("'type'"));
        assert!(
            err(
                "pcli2_asset_metadata_create",
                json!({"path": "/a", "name": "n", "value": "v", "type": "date"})
            )
            .contains("text, number, boolean, url")
        );
    }

    #[test]
    fn test_type_validation() {
        assert!(
            err("pcli2_tenant_list", json!({"pretty": "maybe"})).contains("expected a boolean")
        );
        assert!(
            err("pcli2_tenant_list", json!({"format": ["json"]})).contains("expected a string")
        );
        assert!(
            err(
                "pcli2_asset_geometric_match",
                json!({"uuid": "u", "threshold": "high"})
            )
            .contains("expected a number")
        );
        assert!(
            err("pcli2_folder_dependencies", json!({"folder_path": [1, {}]}))
                .contains("expected a string")
        );
        assert!(err("pcli2_tenant_list", json!([])).contains("must be a JSON object"));
    }

    #[test]
    fn test_lenient_coercions() {
        // String-typed booleans and numbers are accepted, as LLM clients often send them.
        assert_eq!(
            ok(
                "pcli2_tenant_list",
                json!({"pretty": "true", "headers": "false"})
            ),
            v(&["tenant", "list", "--pretty"])
        );
        assert_eq!(
            ok(
                "pcli2_asset_geometric_match",
                json!({"uuid": "u", "threshold": "85.5"})
            ),
            v(&[
                "asset",
                "geometric-match",
                "--uuid",
                "u",
                "--threshold",
                "85.5"
            ])
        );
        assert_eq!(
            ok(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": "3"})
            ),
            v(&[
                "folder",
                "download",
                "--folder-path",
                "/a",
                "--concurrent",
                "3"
            ])
        );
        assert_eq!(
            ok("pcli2_user_get", json!({"user_id": 42})),
            v(&["user", "get", "42"])
        );
        // Whole-number floats are accepted for integers.
        assert_eq!(
            ok(
                "pcli2_folder_download",
                json!({"folder_path": "/a", "concurrent": 3.0})
            ),
            v(&[
                "folder",
                "download",
                "--folder-path",
                "/a",
                "--concurrent",
                "3"
            ])
        );
    }

    #[test]
    fn test_false_flags_and_unknown_keys_ignored() {
        assert_eq!(
            ok(
                "pcli2_tenant_list",
                json!({"pretty": false, "bogus": 1, "headers": null})
            ),
            v(&["tenant", "list"])
        );
        assert_eq!(ok("pcli2_tenant_list", Value::Null), v(&["tenant", "list"]));
    }

    #[test]
    fn test_string_list_trimming() {
        assert_eq!(
            ok(
                "pcli2_asset_metadata_delete",
                json!({"uuid": "u", "name": [" A ", "", "B"]})
            ),
            v(&[
                "asset", "metadata", "delete", "--uuid", "u", "--name", "A", "--name", "B"
            ])
        );
    }

    #[test]
    fn test_local_args_not_forwarded() {
        assert_eq!(
            ok(
                "pcli2_asset_thumbnail",
                json!({"uuid": "u", "response_mode": "url"})
            ),
            v(&["asset", "thumbnail", "--uuid", "u"])
        );
        assert!(
            err(
                "pcli2_asset_thumbnail",
                json!({"uuid": "u", "response_mode": "png"})
            )
            .contains("'response_mode'")
        );
    }

    #[test]
    fn test_uuid_and_path_both_forwarded() {
        // pcli2 itself rejects the conflict; the wrapper forwards what it was given.
        assert_eq!(
            ok("pcli2_asset_get", json!({"uuid": "u", "path": "/p"})),
            v(&["asset", "get", "--uuid", "u", "--path", "/p"])
        );
    }
}
