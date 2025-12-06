module.exports = {
    extends: ["config:recommended"],
    enabled: true,

    autoApprove: true,
    automerge: true,
    automergeStrategy: "squash",
    gitAuthor: "Renovate Bot <bot@renovateapp.com>",
    platform: "github",
    repositories: ["Terrazzo-Web/Terminal"],
    packageRules: [
        {
            description: "lockFileMaintenance",
            matchUpdateTypes: ["pin", "digest", "patch", "minor", "major", "lockFileMaintenance"]
        },
        {
            matchPackageNames: ["/^.*$/"],
            automerge: true
        }
    ]
};
