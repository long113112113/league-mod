import { useState, useEffect, useRef } from "react";
import { api } from "@/lib/tauri";

interface SkinImageProps {
    championId: number;
    skinId: number;
    alt: string;
    className?: string;
    placeholder?: React.ReactNode;
    lazyFetch?: boolean;
}

export function SkinImage({ championId, skinId, alt, className, placeholder, lazyFetch = false }: SkinImageProps) {
    const [src, setSrc] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const containerRef = useRef<HTMLDivElement>(null);
    const [shouldFetch, setShouldFetch] = useState(!lazyFetch);

    useEffect(() => {
        if (!lazyFetch) return;

        const observer = new IntersectionObserver(([entry]) => {
            if (entry.isIntersecting) {
                setShouldFetch(true);
                observer.disconnect();
            }
        });

        if (containerRef.current) {
            observer.observe(containerRef.current);
        }

        return () => observer.disconnect();
    }, [lazyFetch]);

    useEffect(() => {
        if (!shouldFetch) return;

        let mounted = true;
        setLoading(true);

        api.getSkinImage(championId, skinId)
            .then((result) => {
                if (mounted) {
                    if (result.ok) {
                        setSrc(result.value);
                    }
                    setLoading(false);
                }
            })
            .catch(() => {
                if (mounted) {
                    setLoading(false);
                }
            });

        return () => {
            mounted = false;
        };
    }, [championId, skinId, shouldFetch]);

    if (!shouldFetch || loading) {
        return (
            <div ref={containerRef} className={`animate-pulse bg-night-700 ${className}`}>
                {placeholder}
            </div>
        );
    }

    if (!src) {
        return (
            <div className={`bg-night-800 ${className} flex items-center justify-center text-surface-600`}>
                {placeholder || "No Image"}
            </div>
        );
    }

    return (
        <img
            src={src}
            alt={alt}
            className={className}
            loading="lazy"
        />
    );
}
