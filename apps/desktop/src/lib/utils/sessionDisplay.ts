import type { SessionData } from "$lib/types";

const CODEX_EXTERNAL_ID_PATTERN = /^(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})-([a-f0-9]{8})/i;
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function compactWhitespace(value: string | null | undefined): string {
  if (!value) return "";
  return value.replaceAll(/\s+/g, " ").trim();
}

function projectFromDashedPath(project: string): string {
  const markers = ["-Desktop-", "-Projects-"];

  for (const marker of markers) {
    const index = project.indexOf(marker);
    if (index !== -1) {
      return project.slice(index + marker.length);
    }
  }

  const parts = project.split("-").filter(Boolean);
  return parts.at(-1) ?? project;
}

export function getDisplayProject(project: string | null | undefined): string {
  const value = compactWhitespace(project);
  if (!value) return "No project";

  const gitIndex = value.indexOf(".git/");
  if (gitIndex !== -1) {
    const repoPart = value.slice(0, gitIndex);
    const branchPart = value.slice(gitIndex + 5);
    const repoName = repoPart.split("/").findLast(Boolean) ?? repoPart;
    return branchPart ? `${repoName} [${branchPart}]` : repoName;
  }

  if (value.startsWith("-Users-")) {
    return projectFromDashedPath(value);
  }

  if (value.startsWith("/")) {
    const parts = value.split("/").filter(Boolean);
    return parts.at(-1) ?? value;
  }

  return value;
}

export function parseProjectTitle(project: string | null | undefined): { name: string; branch?: string } {
  const value = compactWhitespace(project);
  if (!value) return { name: "No project" };

  const gitIndex = value.indexOf(".git/");
  if (gitIndex !== -1) {
    const repoPart = value.slice(0, gitIndex);
    const branchPart = value.slice(gitIndex + 5);
    const repoName = repoPart.split("/").findLast(Boolean) ?? repoPart;
    return { name: repoName, branch: branchPart };
  }

  if (value.startsWith("-Users-")) {
    return { name: projectFromDashedPath(value) };
  }

  if (value.startsWith("/")) {
    const parts = value.split("/").filter(Boolean);
    return { name: parts.at(-1) ?? value };
  }

  return { name: value };
}

export function getDisplayExternalId(source: string, externalId: string): string {
  const value = compactWhitespace(externalId);
  if (!value) return "unknown";

  const codexMatch = value.match(CODEX_EXTERNAL_ID_PATTERN);
  if ((source.toLowerCase() === "codex" || codexMatch) && codexMatch) {
    const [, date, hour, minute, , token] = codexMatch;
    return `${date} ${hour}:${minute} · ${token}`;
  }

  if (value.startsWith("ses_")) {
    return `ses_${value.slice(4, 12)}`;
  }

  if (UUID_PATTERN.test(value)) {
    return value.slice(0, 8);
  }

  if (value.length <= 16) {
    return value;
  }

  return `${value.slice(0, 16)}...`;
}

export function getSessionSlug(source: string, externalId: string): string {
  const value = compactWhitespace(externalId);
  if (!value) return "session";

  const codexMatch = value.match(CODEX_EXTERNAL_ID_PATTERN);
  if ((source.toLowerCase() === "codex" || codexMatch) && codexMatch) {
    const [, date, hour, minute, , token] = codexMatch;
    return `${date}-${hour}${minute}-${token}`;
  }

  if (value.startsWith("ses_")) {
    return `ses-${value.slice(4, 12)}`;
  }

  if (UUID_PATTERN.test(value)) {
    return value.slice(0, 8);
  }

  return value
    .toLowerCase()
    .replaceAll(/[^a-z0-9_-]+/g, "-")
    .replaceAll(/-+/g, "-")
    .replaceAll(/^-|-$/g, "")
    .slice(0, 24);
}

export function getDisplaySessionTitle(session: SessionData): string {
  const title = compactWhitespace(session.title);
  if (title) return title;

  const project = getDisplayProject(session.project);
  if (project !== "No project") {
    return `${project} session`;
  }

  return `Session ${getDisplayExternalId(session.source, session.external_id)}`;
}
