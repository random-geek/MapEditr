-- THIS MOD IS NOT USEFUL TO USERS!

-- test_mod defines a few nodes and entities which may be used to generate test
-- map data for MapEditr.


local tex = "test_mod_test.png"
local colors = {
	"#FF0000", "#FFFF00", "#00FF00", "#00FFFF", "#0000FF", "#FF00FF"
}


minetest.register_node("test_mod:stone", {
	description = "test_mod stone",
	drawtype = "normal",
	tiles = {"default_stone.png^[colorize:#3FFF3F:63"},
	groups = {oddly_breakable_by_hand = 3},
})


minetest.register_node("test_mod:metadata", {
	description = "test_mod metadata",
	drawtype = "normal",
	tiles = {"default_stone.png^[colorize:#FF3F3F:63"},
	groups = {oddly_breakable_by_hand = 3},

	on_construct = function(pos)
		local meta = minetest.get_meta(pos)
		meta:set_string("formspec",
			"size[8,5]" ..
			"list[current_name;main;0,0;1,1;]" ..
			"list[current_player;main;0,1;8,4;]")
		meta:set_string("infotext", "Test Chest")
		local inv = meta:get_inventory()
		inv:set_size("main", 1)
	end,
})


minetest.register_node("test_mod:timer", {
	description = "test_mod timer",
	drawtype = "nodebox",
	node_box = {
		type = "fixed",
		fixed = {-1/4, -1/2, -1/4, 1/4, 1/4, 1/4}
	},
	tiles = {tex},
	paramtype = "light",
	paramtype2 = "facedir",
	groups = {oddly_breakable_by_hand = 3},

	on_construct = function(pos)
		minetest.get_node_timer(pos):start(1.337)
	end,

	on_timer = function(pos, elapsed)
		local node = minetest.get_node(pos)
		node.param2 = (node.param2 + 4) % 24

		minetest.set_node(pos, node)
		minetest.get_node_timer(pos):start(1.337)
	end,
})


minetest.register_entity("test_mod:color_entity", {
	initial_properties = {
		visual = "cube",
		textures = {tex, tex, tex, tex, tex, tex},
	},

	on_activate = function(self, staticdata, dtime_s)
		if staticdata and staticdata ~= "" then
			t = minetest.deserialize(staticdata)
			self._color_num = t.color_num
		else
			self._color_num = math.random(1, #colors)
		end

		self.object:settexturemod(
			"^[colorize:" .. colors[self._color_num] .. ":127")
	end,

	get_staticdata = function(self)
		return minetest.serialize({color_num = self._color_num})
	end,
})


minetest.register_entity("test_mod:nametag_entity", {
	initial_properties = {
		visual = "sprite",
		textures = {tex},
	},

	on_activate = function(self, staticdata, dtime_s)
		if staticdata and staticdata ~= "" then
			self._text = staticdata
		else
			self._text = tostring(math.random(0, 999999))
		end

		self.object:set_nametag_attributes({
			text = self._text,
			color = "#FFFF00"
		})
	end,

	get_staticdata = function(self)
		return self._text
	end,
})
