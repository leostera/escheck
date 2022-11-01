const ffi = (name, args) => Deno.core.opSync(name, args);

const panic = x => {
	if (typeof x === "object") {
		x = JSON.stringify(x, null, 2);
	}
	throw new Error(x);
};


const EsCheck = {};

EsCheck.__ALL_RULES__ = {};

EsCheck.Rule = {};

EsCheck.Rule.new = (spec) => {
  EsCheck.__ALL_RULES__[spec.name] = spec;
  ffi("op_escheck_rule_new", spec)
}
