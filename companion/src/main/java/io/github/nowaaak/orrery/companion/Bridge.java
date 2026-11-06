package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.Writer;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.function.Consumer;

public final class Bridge implements Runnable {
    private static final int PROTOCOL_VERSION = 1;
    private static final int CONNECT_TIMEOUT_MS = 3000;
    private static final long RETRY_DELAY_MS = 5000;
    private static final int MAX_ATTEMPTS = 20;

    private final String host;
    private final int port;
    private final String token;
    private final Consumer<JsonObject> handler;
    private volatile Writer writer;
    private volatile boolean connected;

    private Bridge(String host, int port, String token, Consumer<JsonObject> handler) {
        this.host = host;
        this.port = port;
        this.token = token;
        this.handler = handler;
    }

    public static Bridge fromSystemProperties(Consumer<JsonObject> handler) {
        String address = System.getProperty("orrery.bridge");
        String token = System.getProperty("orrery.bridge.token");
        if (address == null || token == null) {
            return null;
        }
        int separator = address.lastIndexOf(':');
        if (separator <= 0) {
            return null;
        }
        String host = address.substring(0, separator);
        if (!host.equals("127.0.0.1") && !host.equals("localhost")) {
            OrreryCompanion.LOGGER.warn("Ignoring a bridge address that is not local");
            return null;
        }
        try {
            int port = Integer.parseInt(address.substring(separator + 1));
            return new Bridge(host, port, token, handler);
        } catch (NumberFormatException e) {
            return null;
        }
    }

    public void start() {
        Thread thread = new Thread(this, "Orrery bridge");
        thread.setDaemon(true);
        thread.start();
    }

    public boolean isConnected() {
        return connected;
    }

    public void send(JsonObject message) {
        Writer current = writer;
        if (current == null) {
            return;
        }
        synchronized (this) {
            try {
                current.write(message.toString());
                current.write('\n');
                current.flush();
            } catch (IOException e) {
                OrreryCompanion.LOGGER.debug("Could not send to the launcher", e);
            }
        }
    }

    @Override
    public void run() {
        for (int attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
            try (Socket socket = new Socket()) {
                socket.connect(new InetSocketAddress(host, port), CONNECT_TIMEOUT_MS);
                Writer out = new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8);
                BufferedReader in = new BufferedReader(new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
                JsonObject hello = new JsonObject();
                hello.addProperty("type", "hello");
                hello.addProperty("version", PROTOCOL_VERSION);
                hello.addProperty("token", token);
                hello.addProperty("mod_version", OrreryCompanion.VERSION);
                out.write(hello.toString());
                out.write('\n');
                out.flush();
                String line;
                while ((line = in.readLine()) != null) {
                    JsonObject message = JsonParser.parseString(line).getAsJsonObject();
                    String type = message.has("type") ? message.get("type").getAsString() : "";
                    if (type.equals("welcome")) {
                        writer = out;
                        connected = true;
                        attempt = 0;
                        OrreryCompanion.LOGGER.info("Connected to the Orrery launcher");
                        handler.accept(message);
                    } else {
                        handler.accept(message);
                    }
                }
            } catch (IOException | RuntimeException e) {
                OrreryCompanion.LOGGER.debug("Orrery bridge connection failed", e);
            } finally {
                writer = null;
                connected = false;
            }
            try {
                Thread.sleep(RETRY_DELAY_MS);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return;
            }
        }
    }
}
