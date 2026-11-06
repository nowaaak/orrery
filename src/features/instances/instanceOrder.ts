import type { Instance } from "@/bindings/Instance";
import type { InstanceSort } from "@/bindings/InstanceSort";

export type InstanceSection =
    | { kind: "pinned"; instances: Instance[] }
    | { kind: "group"; name: string; instances: Instance[] }
    | { kind: "ungrouped"; instances: Instance[] };

const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

function compare(sort: InstanceSort): (a: Instance, b: Instance) => number {
    const byName = (a: Instance, b: Instance) => collator.compare(a.name, b.name);
    switch (sort) {
        case "name":
            return byName;
        case "created":
            return (a, b) => b.createdAt - a.createdAt || byName(a, b);
        case "playtime":
            return (a, b) => b.totalPlaySecs - a.totalPlaySecs || byName(a, b);
        case "last_played":
            return (a, b) => (b.lastPlayedAt ?? -1) - (a.lastPlayedAt ?? -1) || byName(a, b);
    }
}

export function sortInstances(instances: Instance[], sort: InstanceSort): Instance[] {
    return [...instances].sort(compare(sort));
}

export function instanceSections(instances: Instance[], sort: InstanceSort): InstanceSection[] {
    const sorted = sortInstances(instances, sort);
    const sections: InstanceSection[] = [];
    const pinned = sorted.filter((instance) => instance.pinned);
    if (pinned.length > 0) {
        sections.push({ kind: "pinned", instances: pinned });
    }
    const groups = new Map<string, Instance[]>();
    const ungrouped: Instance[] = [];
    for (const instance of sorted) {
        if (instance.pinned) {
            continue;
        }
        if (instance.group) {
            groups.set(instance.group, [...(groups.get(instance.group) ?? []), instance]);
        } else {
            ungrouped.push(instance);
        }
    }
    for (const name of [...groups.keys()].sort(collator.compare)) {
        sections.push({ kind: "group", name, instances: groups.get(name)! });
    }
    if (ungrouped.length > 0) {
        sections.push({ kind: "ungrouped", instances: ungrouped });
    }
    return sections;
}

export function groupNames(instances: Instance[]): string[] {
    return [
        ...new Set(instances.flatMap((instance) => (instance.group ? [instance.group] : []))),
    ].sort(collator.compare);
}
