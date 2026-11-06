package io.github.nowaaak.orrery.companion.modules;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import io.github.nowaaak.orrery.companion.JsonFiles;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;

public final class ModuleSettings {
    public static final float MIN_SCALE = 0.5f;
    public static final float MAX_SCALE = 2.5f;
    private static final String FILE_NAME = "modules.json";
    private static final Map<String, ModuleSettings> ENTRIES = new HashMap<>();
    private static JsonObject stored;

    public boolean enabled;
    public float x;
    public float y;
    public float scale = 1f;
    private final Map<String, Integer> options = new HashMap<>();

    private ModuleSettings() {
    }

    public static ModuleSettings get(Module module) {
        return ENTRIES.computeIfAbsent(module.id(), id -> load(module));
    }

    public static void setEnabled(Module module, boolean enabled) {
        get(module).enabled = enabled;
        save();
    }

    public int option(String key, int fallback) {
        return options.getOrDefault(key, fallback);
    }

    public static void setOption(Module module, String key, int value) {
        get(module).options.put(key, value);
        save();
    }

    public static void save() {
        JsonObject root = stored == null ? new JsonObject() : stored.deepCopy();
        for (Map.Entry<String, ModuleSettings> entry : ENTRIES.entrySet()) {
            ModuleSettings settings = entry.getValue();
            JsonObject json = new JsonObject();
            json.addProperty("enabled", settings.enabled);
            json.addProperty("x", settings.x);
            json.addProperty("y", settings.y);
            json.addProperty("scale", settings.scale);
            if (!settings.options.isEmpty()) {
                JsonObject values = new JsonObject();
                settings.options.forEach(values::addProperty);
                json.add("options", values);
            }
            root.add(entry.getKey(), json);
        }
        stored = root;
        JsonFiles.write(file(), root);
    }

    private static ModuleSettings load(Module module) {
        if (stored == null) {
            JsonObject root = JsonFiles.read(file());
            stored = root == null ? new JsonObject() : root;
        }
        ModuleSettings settings = new ModuleSettings();
        settings.enabled = module.enabledByDefault();
        if (module instanceof HudModule hud) {
            settings.x = hud.defaultX();
            settings.y = hud.defaultY();
        }
        JsonElement element = stored.get(module.id());
        if (element != null && element.isJsonObject()) {
            JsonObject json = element.getAsJsonObject();
            settings.enabled = JsonFiles.bool(json, "enabled", settings.enabled);
            settings.x = fraction(json, "x", settings.x);
            settings.y = fraction(json, "y", settings.y);
            settings.scale = clampScale(number(json, "scale", settings.scale));
            JsonElement values = json.get("options");
            if (values != null && values.isJsonObject()) {
                for (Map.Entry<String, JsonElement> value : values.getAsJsonObject().entrySet()) {
                    if (value.getValue().isJsonPrimitive() && value.getValue().getAsJsonPrimitive().isNumber()) {
                        settings.options.put(value.getKey(), value.getValue().getAsInt());
                    }
                }
            }
        }
        return settings;
    }

    public static float clampScale(float scale) {
        return Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale));
    }

    private static float fraction(JsonObject json, String key, float fallback) {
        return Math.max(0f, Math.min(1f, number(json, key, fallback)));
    }

    private static float number(JsonObject json, String key, float fallback) {
        JsonElement element = json.get(key);
        if (element == null || !element.isJsonPrimitive() || !element.getAsJsonPrimitive().isNumber()) {
            return fallback;
        }
        float value = element.getAsFloat();
        return Float.isFinite(value) ? value : fallback;
    }

    private static Path file() {
        return JsonFiles.config(FILE_NAME);
    }
}
