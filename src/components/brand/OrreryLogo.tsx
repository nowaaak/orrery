import { useId } from "react";

export function OrreryLogo({ size = 32, className }: { size?: number; className?: string }) {
    const id = useId().replace(/:/g, "");
    const sky = `sky-${id}`;
    const sun = `sun-${id}`;
    const ringA = `ringA-${id}`;
    const ringB = `ringB-${id}`;

    return (
        <svg
            viewBox="0 0 1024 1024"
            width={size}
            height={size}
            className={className}
            role="img"
            aria-label="Orrery"
        >
            <defs>
                <radialGradient id={sky} cx="50%" cy="38%" r="75%">
                    <stop offset="0" stopColor="#2a2f7a" />
                    <stop offset="0.55" stopColor="#151a45" />
                    <stop offset="1" stopColor="#0a0d24" />
                </radialGradient>
                <radialGradient id={sun} cx="42%" cy="38%" r="65%">
                    <stop offset="0" stopColor="#fff4c2" />
                    <stop offset="0.45" stopColor="#fbbf24" />
                    <stop offset="1" stopColor="#d97706" />
                </radialGradient>
                <linearGradient id={ringA} x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0" stopColor="#a5b4fc" />
                    <stop offset="1" stopColor="#6366f1" />
                </linearGradient>
                <linearGradient id={ringB} x1="1" y1="0" x2="0" y2="1">
                    <stop offset="0" stopColor="#67e8f9" />
                    <stop offset="1" stopColor="#0891b2" />
                </linearGradient>
            </defs>
            <rect x="32" y="32" width="960" height="960" rx="224" fill={`url(#${sky})`} />
            <ellipse
                cx="512"
                cy="512"
                rx="370"
                ry="150"
                fill="none"
                stroke={`url(#${ringA})`}
                strokeWidth="40"
                transform="rotate(-24 512 512)"
            />
            <ellipse
                cx="512"
                cy="512"
                rx="300"
                ry="112"
                fill="none"
                stroke={`url(#${ringB})`}
                strokeWidth="36"
                transform="rotate(32 512 512)"
            />
            <circle cx="512" cy="512" r="140" fill={`url(#${sun})`} />
            <circle cx="845" cy="362" r="64" fill="#818cf8" stroke="#0a0d24" strokeWidth="16" />
            <circle cx="282" cy="692" r="52" fill="#22d3ee" stroke="#0a0d24" strokeWidth="14" />
        </svg>
    );
}
