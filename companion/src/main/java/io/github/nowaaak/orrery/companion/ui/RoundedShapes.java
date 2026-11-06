package io.github.nowaaak.orrery.companion.ui;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class RoundedShapes {
    private static final int CACHE_SIZE = 512;
    private static final int LEVELS = 64;
    private static final Map<Key, List<Run>> CACHE = new LinkedHashMap<>(64, 0.75f, true) {
        @Override
        protected boolean removeEldestEntry(Map.Entry<Key, List<Run>> eldest) {
            return size() > CACHE_SIZE;
        }
    };

    private RoundedShapes() {
    }

    public record Run(int x0, int y0, int x1, int y1, float coverage) {
    }

    private record Key(int width, int height, float radius, int border) {
    }

    public static List<Run> runs(int width, int height, float radius, int border) {
        return CACHE.computeIfAbsent(new Key(width, height, radius, border), RoundedShapes::build);
    }

    private static List<Run> build(Key key) {
        List<Run> result = new ArrayList<>();
        List<int[]> previous = null;
        int previousStart = 0;
        for (int y = 0; y < key.height; y++) {
            List<int[]> row = row(key, y);
            if (previous != null && sameRuns(previous, row)) {
                continue;
            }
            if (previous != null) {
                flush(result, previous, previousStart, y);
            }
            previous = row;
            previousStart = y;
        }
        if (previous != null) {
            flush(result, previous, previousStart, key.height);
        }
        return List.copyOf(result);
    }

    private static List<int[]> row(Key key, int y) {
        List<int[]> runs = new ArrayList<>();
        int start = -1;
        int level = 0;
        for (int x = 0; x <= key.width; x++) {
            int current = x == key.width ? 0 : quantize(coverage(key, x + 0.5f, y + 0.5f));
            if (current != level) {
                if (level > 0) {
                    runs.add(new int[] {start, x, level});
                }
                start = x;
                level = current;
            }
        }
        return runs;
    }

    private static boolean sameRuns(List<int[]> a, List<int[]> b) {
        if (a.size() != b.size()) {
            return false;
        }
        for (int i = 0; i < a.size(); i++) {
            int[] left = a.get(i);
            int[] right = b.get(i);
            if (left[0] != right[0] || left[1] != right[1] || left[2] != right[2]) {
                return false;
            }
        }
        return true;
    }

    private static void flush(List<Run> result, List<int[]> runs, int y0, int y1) {
        for (int[] run : runs) {
            result.add(new Run(run[0], y0, run[1], y1, run[2] / (float) LEVELS));
        }
    }

    private static int quantize(float coverage) {
        return Math.round(Math.max(0f, Math.min(1f, coverage)) * LEVELS);
    }

    private static float coverage(Key key, float x, float y) {
        float outer = box(x, y, key.width, key.height, key.radius);
        if (key.border <= 0) {
            return outer;
        }
        float inner = box(x - key.border, y - key.border, key.width - key.border * 2f, key.height - key.border * 2f,
                Math.max(0f, key.radius - key.border));
        return Math.max(0f, outer - inner);
    }

    private static float box(float x, float y, float width, float height, float radius) {
        if (width <= 0 || height <= 0) {
            return 0f;
        }
        float qx = Math.abs(x - width / 2f) - (width / 2f - radius);
        float qy = Math.abs(y - height / 2f) - (height / 2f - radius);
        float outside = (float) Math.hypot(Math.max(qx, 0f), Math.max(qy, 0f));
        float inside = Math.min(Math.max(qx, qy), 0f);
        return Math.max(0f, Math.min(1f, 0.5f - (outside + inside - radius)));
    }
}
