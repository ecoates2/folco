import { plugin } from "bun";
import { compile, compileModule } from "svelte/compiler";

// compileModule doesn't accept TypeScript, so strip types first.
const ts = new Bun.Transpiler({ loader: "ts" });

plugin({
	name: "svelte",
	setup(build) {
		build.onLoad({ filter: /\.svelte(\.(ts|js))?$/ }, async ({ path }) => {
			const source = await Bun.file(path).text();
			const isRuneModule = /\.svelte\.(ts|js)$/.test(path);

			const { js } = isRuneModule
				? compileModule(ts.transformSync(source), { filename: path, generate: "client" })
				: compile(source, { filename: path, generate: "client" });

			return { contents: js.code, loader: "js" };
		});
	},
});
