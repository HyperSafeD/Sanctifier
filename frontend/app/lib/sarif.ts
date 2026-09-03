import type { Finding, Severity } from "../types";

interface SarifResult {
  ruleId: string;
  level: string;
  message: { text: string };
  locations: Array<{
    physicalLocation: {
      artifactLocation: { uri: string };
      region?: { startLine: number };
    };
  }>;
  properties?: Record<string, unknown>;
}

interface SarifRun {
  tool: {
    driver: {
      name: string;
      version: string;
      informationUri: string;
      rules: Array<{
        id: string;
        name: string;
        shortDescription: { text: string };
        fullDescription: { text: string };
        defaultConfiguration: { level: string };
        properties: { category: string };
      }>;
    };
  };
  results: SarifResult[];
}

interface SarifLog {
  $schema: string;
  version: string;
  runs: SarifRun[];
}

const severityToSarifLevel: Record<Severity, string> = {
  critical: "error",
  high: "error",
  medium: "warning",
  low: "note",
};

export function findingsToSarif(
  findings: Finding[],
  toolVersion = "1.0.0",
  informationUri = "https://github.com/HyperSafeD/Sanctifier"
): SarifLog {
  const rulesMap = new Map<string, { id: string; name: string; description: string; category: string; level: string }>();

  const results: SarifResult[] = findings.map((f) => {
    if (!rulesMap.has(f.code)) {
      rulesMap.set(f.code, {
        id: f.code,
        name: f.title,
        description: f.suggestion || f.title,
        category: f.category,
        level: severityToSarifLevel[f.severity] || "warning",
      });
    }

    const result: SarifResult = {
      ruleId: f.code,
      level: severityToSarifLevel[f.severity] || "warning",
      message: { text: `${f.title} at ${f.location}` },
      locations: [
        {
          physicalLocation: {
            artifactLocation: { uri: f.location.split(":")[0] || f.location },
          },
        },
      ],
      properties: {
        category: f.category,
        severity: f.severity,
        location: f.location,
      },
    };

    if (f.suggestion) {
      result.properties!["suggestion"] = f.suggestion;
    }
    if (f.line) {
      result.locations[0].physicalLocation.region = { startLine: f.line };
    }

    return result;
  });

  const rules = Array.from(rulesMap.entries()).map(([id, rule]) => ({
    id,
    name: rule.name,
    shortDescription: { text: rule.description },
    fullDescription: { text: rule.description },
    defaultConfiguration: { level: rule.level },
    properties: { category: rule.category },
  }));

  return {
    $schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
    version: "2.1.0",
    runs: [
      {
        tool: {
          driver: {
            name: "Sanctifier",
            version: toolVersion,
            informationUri,
            rules,
          },
        },
        results,
      },
    ],
  };
}
