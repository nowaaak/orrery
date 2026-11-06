package io.github.nowaaak.orrery.companion.modules.game;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import io.github.nowaaak.orrery.companion.JsonFiles;
import java.util.ArrayList;
import java.util.List;

public final class Waypoints {
    public record Waypoint(String name, int x, int y, int z, String dimension, boolean death) {
    }

    private static final String FILE_NAME = "waypoints.json";
    private static final int MAX_NAME = 32;
    private static final int MAX_PER_CONTEXT = 200;
    private static final int LIMIT = 30_000_000;
    private static JsonObject root;

    private Waypoints() {
    }

    public static List<Waypoint> list(String context) {
        List<Waypoint> result = new ArrayList<>();
        if (context == null) {
            return result;
        }
        JsonElement entries = load().get(context);
        if (entries == null || !entries.isJsonArray()) {
            return result;
        }
        for (JsonElement element : entries.getAsJsonArray()) {
            Waypoint waypoint = parse(element);
            if (waypoint != null && result.size() < MAX_PER_CONTEXT) {
                result.add(waypoint);
            }
        }
        return result;
    }

    public static void add(String context, Waypoint waypoint) {
        List<Waypoint> current = list(context);
        if (context == null || current.size() >= MAX_PER_CONTEXT) {
            return;
        }
        current.add(waypoint);
        store(context, current);
    }

    public static void remove(String context, Waypoint waypoint) {
        List<Waypoint> current = list(context);
        if (current.remove(waypoint)) {
            store(context, current);
        }
    }

    public static void setDeath(String context, Waypoint waypoint) {
        List<Waypoint> current = list(context);
        current.removeIf(Waypoint::death);
        current.add(waypoint);
        store(context, current);
    }

    public static String cleanName(String name) {
        String trimmed = name == null ? "" : name.strip().replaceAll("\\p{Cntrl}", "");
        return trimmed.length() > MAX_NAME ? trimmed.substring(0, MAX_NAME) : trimmed;
    }

    private static Waypoint parse(JsonElement element) {
        if (!element.isJsonObject()) {
            return null;
        }
        JsonObject json = element.getAsJsonObject();
        String name = cleanName(JsonFiles.string(json, "name"));
        String dimension = JsonFiles.string(json, "dimension");
        Integer x = coordinate(json, "x");
        Integer y = coordinate(json, "y");
        Integer z = coordinate(json, "z");
        if (name.isEmpty() || dimension == null || !dimension.matches("[a-z0-9_.-]+:[a-z0-9_./-]+") || x == null || y == null || z == null) {
            return null;
        }
        return new Waypoint(name, x, y, z, dimension, JsonFiles.bool(json, "death", false));
    }

    private static Integer coordinate(JsonObject json, String key) {
        JsonElement value = json.get(key);
        if (value == null || !value.isJsonPrimitive() || !value.getAsJsonPrimitive().isNumber()) {
            return null;
        }
        double number = value.getAsDouble();
        return Math.abs(number) <= LIMIT ? (int) number : null;
    }

    private static void store(String context, List<Waypoint> waypoints) {
        JsonArray array = new JsonArray();
        for (Waypoint waypoint : waypoints) {
            JsonObject json = new JsonObject();
            json.addProperty("name", waypoint.name());
            json.addProperty("x", waypoint.x());
            json.addProperty("y", waypoint.y());
            json.addProperty("z", waypoint.z());
            json.addProperty("dimension", waypoint.dimension());
            if (waypoint.death()) {
                json.addProperty("death", true);
            }
            array.add(json);
        }
        load().add(context, array);
        JsonFiles.write(JsonFiles.config(FILE_NAME), root);
    }

    private static JsonObject load() {
        if (root == null) {
            JsonObject stored = JsonFiles.read(JsonFiles.config(FILE_NAME));
            root = stored == null ? new JsonObject() : stored;
        }
        return root;
    }
}
