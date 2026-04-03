.PHONY: build generate-types dev-backend dev-frontend

# Re-run generate-types whenever RenderParams or any other Rust struct that is
# shared with the frontend changes.  The test writes the resulting TypeScript
# files into frontend/src/bindings/ so the frontend always matches the backend.
generate-types:
	@echo "==> Generating TypeScript bindings from Rust structs…"
	cd backend && cargo test --test generate_bindings
	@echo "==> Bindings written to frontend/src/bindings/"

# Production: generate types, build frontend, then backend — single binary.
# Run from the project root:  ./backend/target/release/ascii-renderer
build: generate-types
	@echo "==> Building frontend…"
	cd frontend && npm run build
	@echo "==> Building backend…"
	cd backend && cargo build --release
	@echo ""
	@echo "Done. Start the app:"
	@echo "  ./backend/target/release/ascii-renderer"

# Development: run each process in its own terminal.
#   Terminal 1:  make dev-backend
#   Terminal 2:  make dev-frontend
dev-backend:
	cd backend && STATIC_DIR=../frontend/dist cargo run

dev-frontend:
	cd frontend && npm run dev
