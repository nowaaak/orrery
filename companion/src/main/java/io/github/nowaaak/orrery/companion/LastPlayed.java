package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import java.nio.file.Path;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.ConnectScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.multiplayer.resolver.ServerAddress;
import net.minecraft.client.server.IntegratedServer;
import net.minecraft.world.level.storage.LevelResource;

public record LastPlayed(String kind, String target, String name) {
    public static final String WORLD = "world";
    public static final String SERVER = "server";
    private static final int MAX_LENGTH = 256;
    private static final String FILE_NAME = "last-played.json";

    private static LastPlayed current;
    private static boolean loaded;

    public static LastPlayed get() {
        if (!loaded) {
            loaded = true;
            JsonObject root = JsonFiles.read(file());
            current = root == null ? null : of(JsonFiles.string(root, "kind"), JsonFiles.string(root, "target"), JsonFiles.string(root, "name"));
        }
        return current;
    }

    public static void remember(Minecraft minecraft) {
        LastPlayed next = capture(minecraft);
        if (next == null || next.equals(get())) {
            return;
        }
        current = next;
        JsonObject root = new JsonObject();
        root.addProperty("kind", next.kind);
        root.addProperty("target", next.target);
        root.addProperty("name", next.name);
        JsonFiles.write(file(), root);
    }

    public boolean isWorld() {
        return WORLD.equals(kind);
    }

    public boolean isAvailable(Minecraft minecraft) {
        return isWorld() ? minecraft.getLevelSource().levelExists(target) : minecraft.allowsMultiplayer();
    }

    public void open(Minecraft minecraft, Screen parent) {
        if (isWorld()) {
            //? if >=1.21.11 {
            minecraft.createWorldOpenFlows().openWorld(target, () -> Screens.open(minecraft, parent));
            //?} else {
            /*minecraft.createWorldOpenFlows().loadLevel(parent, target);
            *///?}
            return;
        }
        if (minecraft.allowsMultiplayer()) {
            connect(minecraft, parent, name, target);
        }
    }

    private static void connect(Minecraft minecraft, Screen parent, String name, String address) {
        //? if >=1.21.11 {
        ServerData data = new ServerData(name, address, ServerData.Type.OTHER);
        ConnectScreen.startConnecting(parent, minecraft, ServerAddress.parseString(address), data, false, null);
        //?} else {
        /*ServerData data = new ServerData(name, address, false);
        ConnectScreen.startConnecting(parent, minecraft, ServerAddress.parseString(address), data, false);
        *///?}
    }

    public static String context(Minecraft minecraft) {
        LastPlayed current = capture(minecraft);
        if (current != null) {
            return current.kind + ":" + current.target;
        }
        ServerData data = minecraft.getCurrentServer();
        return data == null ? null : SERVER + ":" + data.ip;
    }

    private static LastPlayed capture(Minecraft minecraft) {
        if (minecraft.level == null) {
            return null;
        }
        IntegratedServer server = minecraft.getSingleplayerServer();
        if (server != null) {
            Path folder = server.getWorldPath(LevelResource.ROOT).normalize().getFileName();
            return folder == null ? null : of(WORLD, folder.toString(), server.getWorldData().getLevelName());
        }
        ServerData data = minecraft.getCurrentServer();
        if (data == null || data.isLan()) {
            return null;
        }
        //? if >=1.21.11 {
        if (data.isRealm()) {
            return null;
        }
        //?}
        return of(SERVER, data.ip, data.name);
    }

    private static LastPlayed of(String kind, String target, String name) {
        if (!WORLD.equals(kind) && !SERVER.equals(kind)) {
            return null;
        }
        if (target == null || target.isBlank() || target.length() > MAX_LENGTH) {
            return null;
        }
        if (WORLD.equals(kind) && !isPlainFolderName(target)) {
            return null;
        }
        String label = name == null || name.isBlank() ? target : name.strip();
        return new LastPlayed(kind, target, label.length() > MAX_LENGTH ? label.substring(0, MAX_LENGTH) : label);
    }

    private static boolean isPlainFolderName(String folder) {
        return !folder.equals(".") && !folder.equals("..") && folder.chars().noneMatch(c -> c == '/' || c == '\\' || c == ':' || c < 0x20);
    }

    private static Path file() {
        return JsonFiles.config(FILE_NAME);
    }
}
