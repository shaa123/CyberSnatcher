import type { FavoriteItem } from "../../lib/types";

interface Props {
  favorites: FavoriteItem[];
  onNavigate: (url: string) => void;
  onRemove: (id: string) => void;
}

export default function FavoritesDropdown({ favorites, onNavigate, onRemove }: Props) {
  return (
    <div
      style={{
        position: "absolute",
        top: "100%",
        right: 0,
        width: "320px",
        maxHeight: "400px",
        overflowY: "auto",
        background: "var(--panel)",
        border: "1px solid var(--border-purple)",
        borderRadius: "4px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.6)",
        zIndex: 100,
        marginTop: "4px",
      }}
    >
      <div
        style={{
          padding: "8px 12px",
          borderBottom: "1px solid var(--border-purple)",
          fontSize: "10px",
          letterSpacing: "2px",
          color: "var(--purple)",
          fontFamily: "'Orbitron', sans-serif",
          fontWeight: 700,
        }}
      >
        FAVORITES
      </div>
      {favorites.length === 0 ? (
        <div
          style={{
            padding: "20px 12px",
            textAlign: "center",
            fontSize: "11px",
            color: "var(--text-dim)",
          }}
        >
          No favorites yet. Click the star to add one.
        </div>
      ) : (
        favorites.map((fav) => (
          <div
            key={fav.id}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              padding: "8px 12px",
              borderBottom: "1px solid #3a2a5533",
              cursor: "pointer",
              transition: "background 0.15s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "#b400ff11")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.background = "transparent")
            }
            onClick={() => onNavigate(fav.url)}
          >
            <span style={{ color: "#fbbf24", fontSize: "12px", flexShrink: 0 }}>
              ★
            </span>
            <div style={{ flex: 1, overflow: "hidden" }}>
              <div
                style={{
                  fontSize: "11px",
                  color: "var(--text)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {fav.title}
              </div>
              <div
                style={{
                  fontSize: "9px",
                  color: "var(--text-dimmer)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {fav.url}
              </div>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRemove(fav.id);
              }}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-dimmer)",
                cursor: "pointer",
                padding: "2px 4px",
                fontSize: "12px",
                flexShrink: 0,
              }}
              title="Remove"
            >
              ✕
            </button>
          </div>
        ))
      )}
    </div>
  );
}
