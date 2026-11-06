package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import java.nio.file.Path;

public final class Settings {
    private static final String FILE_NAME = "settings.json";
    private static boolean titleScreen = true;
    private static boolean orreryTitleScreen = true;
    private static boolean loaded;

    private Settings() {
    }

    public static boolean titleScreen() {
        load();
        return titleScreen;
    }

    public static void setTitleScreen(boolean enabled) {
        load();
        titleScreen = enabled;
        save();
    }

    public static boolean orreryTitleScreen() {
        load();
        return orreryTitleScreen;
    }

    public static void setOrreryTitleScreen(boolean enabled) {
        load();
        orreryTitleScreen = enabled;
        save();
    }

    private static void load() {
        if (loaded) {
            return;
        }
        loaded = true;
        JsonObject root = JsonFiles.read(file());
        if (root != null) {
            titleScreen = JsonFiles.bool(root, "titleScreen", true);
            orreryTitleScreen = JsonFiles.bool(root, "orreryTitleScreen", true);
        }
    }

    private static void save() {
        JsonObject root = new JsonObject();
        root.addProperty("titleScreen", titleScreen);
        root.addProperty("orreryTitleScreen", orreryTitleScreen);
        JsonFiles.write(file(), root);
    }

    private static Path file() {
        return JsonFiles.config(FILE_NAME);
    }
}
