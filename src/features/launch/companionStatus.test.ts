import { describe, expect, it } from "vitest";
import { companionLocation, dimensionLabel, isOnServer, serverKey } from "./companionStatus";

describe("serverKey", () => {
    it("normalizes case, default port and trailing dots", () => {
        expect(serverKey("Play.Example.org")).toBe("play.example.org");
        expect(serverKey("play.example.org:25565")).toBe("play.example.org");
        expect(serverKey("example.org.")).toBe("example.org");
        expect(serverKey("localhost:25566")).toBe("localhost:25566");
        expect(serverKey("[::1]:25565")).toBe("::1");
        expect(serverKey("[::1]:25570")).toBe("[::1]:25570");
    });
});

describe("companion status", () => {
    const status = {
        screen: "server",
        world: null,
        server: "Play.Example.org",
        dimension: "minecraft:the_nether",
    };

    it("matches the current server", () => {
        expect(isOnServer(status, "play.example.org:25565")).toBe(true);
        expect(isOnServer(status, "other.org")).toBe(false);
        expect(isOnServer(null, "play.example.org")).toBe(false);
    });

    it("names dimensions", () => {
        expect(dimensionLabel("minecraft:the_end")).toBe("End");
        expect(dimensionLabel("aether:the_aether")).toBe("The Aether");
        expect(dimensionLabel("mod:caves/deep_dark")).toBe("Deep Dark");
    });

    it("describes the location", () => {
        expect(companionLocation(status)).toBe("Play.Example.org · Nether");
        expect(
            companionLocation({ screen: "world", world: "Welt", server: null, dimension: null }),
        ).toBe("Welt");
        expect(
            companionLocation({ screen: "menu", world: null, server: null, dimension: null }),
        ).toBeNull();
    });
});
