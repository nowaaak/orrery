import { useEffect, useRef, type KeyboardEvent, type PointerEvent } from "react";
import { cn } from "@/lib/utils";
import { normalizeSkin } from "./normalizeSkin";
import { FLOATS_PER_VERTEX, buildMesh, playerBoxes, type TextureKind } from "./skinModel";

const VERTEX_SHADER = `#version 300 es
in vec3 position;
in vec3 normal;
in vec2 uv;
uniform mat4 projection;
uniform mat4 model;
out vec2 vUv;
out float vLight;
void main() {
    vec3 n = normalize(mat3(model) * normal);
    vLight = 0.62 + 0.38 * max(dot(n, normalize(vec3(0.4, 0.8, 0.9))), 0.0);
    vUv = uv;
    gl_Position = projection * model * vec4(position, 1.0);
}`;

const FRAGMENT_SHADER = `#version 300 es
precision mediump float;
in vec2 vUv;
in float vLight;
uniform sampler2D tex;
out vec4 color;
void main() {
    vec4 texel = texture(tex, vUv);
    if (texel.a < 0.5) discard;
    color = vec4(texel.rgb * vLight, 1.0);
}`;

const IDLE_SPEED = 0.0006;
const DRAG_SPEED = 0.01;
const KEY_STEP = 0.2;

type Matrix = Float32Array;

function perspective(fov: number, aspect: number, near: number, far: number): Matrix {
    const f = 1 / Math.tan(fov / 2);
    const nf = 1 / (near - far);
    return new Float32Array([
        f / aspect,
        0,
        0,
        0,
        0,
        f,
        0,
        0,
        0,
        0,
        (far + near) * nf,
        -1,
        0,
        0,
        2 * far * near * nf,
        0,
    ]);
}

function modelMatrix(yaw: number, pitch: number): Matrix {
    const cy = Math.cos(yaw);
    const sy = Math.sin(yaw);
    const cp = Math.cos(pitch);
    const sp = Math.sin(pitch);
    const scale = 1 / 16;
    return new Float32Array([
        cy * scale,
        sp * sy * scale,
        -cp * sy * scale,
        0,
        0,
        cp * scale,
        sp * scale,
        0,
        sy * scale,
        -sp * cy * scale,
        cp * cy * scale,
        0,
        0,
        -1.0,
        -4.2,
        1,
    ]);
}

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader | null {
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error(gl.getShaderInfoLog(shader));
        return null;
    }
    return shader;
}

type Renderer = {
    gl: WebGL2RenderingContext;
    program: WebGLProgram;
    buffers: Record<TextureKind, WebGLBuffer>;
    textures: Record<TextureKind, WebGLTexture>;
    counts: Record<TextureKind, number>;
    vao: WebGLVertexArrayObject;
};

function createRenderer(canvas: HTMLCanvasElement): Renderer | null {
    const gl = canvas.getContext("webgl2", { antialias: true, premultipliedAlpha: false });
    if (!gl) return null;
    const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
    const fragment = compile(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
    const program = gl.createProgram();
    if (!vertex || !fragment || !program) return null;
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        console.error(gl.getProgramInfoLog(program));
        return null;
    }
    const vao = gl.createVertexArray();
    const skinBuffer = gl.createBuffer();
    const capeBuffer = gl.createBuffer();
    const skinTexture = gl.createTexture();
    const capeTexture = gl.createTexture();
    if (!vao || !skinBuffer || !capeBuffer || !skinTexture || !capeTexture) return null;
    for (const texture of [skinTexture, capeTexture]) {
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    gl.enable(gl.DEPTH_TEST);
    return {
        gl,
        program,
        vao,
        buffers: { skin: skinBuffer, cape: capeBuffer },
        textures: { skin: skinTexture, cape: capeTexture },
        counts: { skin: 0, cape: 0 },
    };
}

function draw(renderer: Renderer, yaw: number, pitch: number) {
    const { gl, program, vao } = renderer;
    const canvas = gl.canvas as HTMLCanvasElement;
    const ratio = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(canvas.clientWidth * ratio));
    const height = Math.max(1, Math.round(canvas.clientHeight * ratio));
    if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.useProgram(program);
    gl.uniformMatrix4fv(
        gl.getUniformLocation(program, "projection"),
        false,
        perspective(Math.PI / 5, width / height, 0.1, 50),
    );
    gl.uniformMatrix4fv(gl.getUniformLocation(program, "model"), false, modelMatrix(yaw, pitch));
    gl.bindVertexArray(vao);
    for (const kind of ["skin", "cape"] as const) {
        if (renderer.counts[kind] === 0) continue;
        gl.bindBuffer(gl.ARRAY_BUFFER, renderer.buffers[kind]);
        const stride = FLOATS_PER_VERTEX * 4;
        const attributes: [string, number, number][] = [
            ["position", 3, 0],
            ["normal", 3, 12],
            ["uv", 2, 24],
        ];
        for (const [name, size, offset] of attributes) {
            const location = gl.getAttribLocation(program, name);
            gl.enableVertexAttribArray(location);
            gl.vertexAttribPointer(location, size, gl.FLOAT, false, stride, offset);
        }
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, renderer.textures[kind]);
        gl.uniform1i(gl.getUniformLocation(program, "tex"), 0);
        gl.drawArrays(gl.TRIANGLES, 0, renderer.counts[kind]);
    }
}

export function SkinViewer({
    skin,
    cape,
    slim,
    label,
    className,
}: {
    skin: Blob | null;
    cape: Blob | null;
    slim: boolean;
    label: string;
    className?: string;
}) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const rendererRef = useRef<Renderer | null>(null);
    const view = useRef({ yaw: 0.5, pitch: 0.12, dragging: false, lastX: 0, lastY: 0 });
    const redraw = useRef<() => void>(() => undefined);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const renderer = createRenderer(canvas);
        rendererRef.current = renderer;
        if (!renderer) return;
        const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
        let frame = 0;
        let last = performance.now();
        const render = () => {
            draw(renderer, view.current.yaw, view.current.pitch);
        };
        const loop = (now: number) => {
            if (!view.current.dragging && !reduced.matches) {
                view.current.yaw += (now - last) * IDLE_SPEED;
            }
            last = now;
            render();
            frame = requestAnimationFrame(loop);
        };
        redraw.current = render;
        frame = requestAnimationFrame(loop);
        return () => {
            cancelAnimationFrame(frame);
            redraw.current = () => undefined;
            rendererRef.current = null;
        };
    }, []);

    useEffect(() => {
        const renderer = rendererRef.current;
        if (!renderer) return;
        let cancelled = false;
        const load = async () => {
            const [skinImage, capeImage] = await Promise.all([
                skin ? normalizeSkin(skin) : null,
                cape ? createImageBitmap(cape) : null,
            ]);
            if (cancelled) return;
            const { gl } = renderer;
            const upload = (kind: TextureKind, image: ImageBitmap | null) => {
                if (!image) return;
                gl.bindTexture(gl.TEXTURE_2D, renderer.textures[kind]);
                gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
            };
            upload("skin", skinImage);
            upload("cape", capeImage);
            const legacy = skinImage !== null && skinImage.height === skinImage.width / 2;
            const mesh = buildMesh(playerBoxes(slim, legacy, capeImage !== null), {
                skin: [skinImage?.width ?? 64, skinImage?.height ?? 64],
                cape: [capeImage?.width ?? 64, capeImage?.height ?? 32],
            });
            for (const kind of ["skin", "cape"] as const) {
                const data = kind === "skin" && !skinImage ? new Float32Array() : mesh[kind];
                gl.bindBuffer(gl.ARRAY_BUFFER, renderer.buffers[kind]);
                gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
                renderer.counts[kind] = data.length / FLOATS_PER_VERTEX;
            }
            redraw.current();
        };
        load().catch((error: unknown) => console.error("could not load the skin", error));
        return () => {
            cancelled = true;
        };
    }, [skin, cape, slim]);

    const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        view.current = {
            ...view.current,
            dragging: true,
            lastX: event.clientX,
            lastY: event.clientY,
        };
    };

    const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
        if (!view.current.dragging) return;
        const state = view.current;
        state.yaw += (event.clientX - state.lastX) * DRAG_SPEED;
        state.pitch = Math.max(
            -0.6,
            Math.min(0.6, state.pitch + (event.clientY - state.lastY) * DRAG_SPEED),
        );
        state.lastX = event.clientX;
        state.lastY = event.clientY;
        redraw.current();
    };

    const onPointerUp = () => {
        view.current.dragging = false;
    };

    const onKeyDown = (event: KeyboardEvent<HTMLCanvasElement>) => {
        const step = { ArrowLeft: -KEY_STEP, ArrowRight: KEY_STEP }[event.key];
        if (step !== undefined) {
            event.preventDefault();
            view.current.yaw += step;
            redraw.current();
        }
    };

    return (
        <canvas
            ref={canvasRef}
            role="img"
            aria-label={label}
            tabIndex={0}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onKeyDown={onKeyDown}
            className={cn(
                "cursor-grab touch-none rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing",
                className,
            )}
        />
    );
}
