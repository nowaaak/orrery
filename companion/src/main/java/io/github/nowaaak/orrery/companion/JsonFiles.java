package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import net.fabricmc.loader.api.FabricLoader;

public final class JsonFiles {
    private JsonFiles() {
    }

    public static Path config(String name) {
        return FabricLoader.getInstance().getConfigDir().resolve(OrreryCompanion.ID).resolve(name);
    }

    public static JsonObject read(Path file) {
        try {
            return JsonParser.parseString(Files.readString(file, StandardCharsets.UTF_8)).getAsJsonObject();
        } catch (NoSuchFileException e) {
            return null;
        } catch (IOException | RuntimeException e) {
            OrreryCompanion.LOGGER.warn("Ignoring an unreadable {}", file, e);
            return null;
        }
    }

    public static void write(Path file, JsonObject root) {
        Path temporary = file.resolveSibling(file.getFileName() + ".tmp");
        try {
            Files.createDirectories(file.getParent());
            Files.writeString(temporary, root.toString(), StandardCharsets.UTF_8);
            Files.move(temporary, file, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException e) {
            OrreryCompanion.LOGGER.warn("Could not save {}", file, e);
        }
    }

    public static String string(JsonObject root, String key) {
        return root.has(key) && root.get(key).isJsonPrimitive() ? root.get(key).getAsString() : null;
    }

    public static boolean bool(JsonObject root, String key, boolean fallback) {
        return root.has(key) && root.get(key).isJsonPrimitive() && root.get(key).getAsJsonPrimitive().isBoolean()
                ? root.get(key).getAsBoolean()
                : fallback;
    }
}
