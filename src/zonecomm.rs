
type HistoricRR {
	RR          rr;
	u32         latest_introduction;
	Option<u32> ttl_countdown_done;
	bool        removed;
};
#OR#NAMED#TREE#
type HistoricRdata {
	Rdata       rdata;
	u32         ttl;
	u32         latest_introduction;
	Option<u32> ttl_countdown_done;
	bool        removed;
};

type ZoneState {
	DNSName     name;
	Option<SOA> rr_soa;
	Vec<RR>     zone;
};

enum ACLTimeConstraint {
	ttl_min   (Option<u32>),
	ttl_max   (Option<u32>),
	ttl_range (Option<u32>,Option<u32>)
};

enum ACLTimeModifier {
	ttl_floor (u32),
	ttl_ceil  (u32)
};

enum ACLStringConstraint {
	str_exact  (String),
	str_regexp (String)
}

enum ACLFieldRule {
	fld_name  (Vec<ACLNameConstraint>,   Vec<ACLNameModifier>  ),
	fld_type  (Vec<ACLTypeConstraint>,   Vec<ACLTypeModifier>  ),
	fld_in                                                      ,
	fld_chaos                                                   ,
	fld_ttl   (Vec<ACLTimeConstraint>,   Vec<ACLTimeModifier>  ),
	fld_rdlen (Vec<ACLIntConstraint>,    Vec<ACLIntModifier>   ),
	fld_u16   (Vec<ACLIntConstraint>,    Vec<ACLIntModifier>   ),
	fld_u32   (Vec<ACLIntConstraint>,    Vec<ACLIntModifier>   ),
	fld_u64   (Vec<ACLIntConstraint>,    Vec<ACLIntModifier>   ),
	fld_u128  (Vec<ACLIntConstraint>,    Vec<ACLIntModifier>   ),
	fld_len8  (Vec<ACLStringConstraint>                        ),
	fld_len16 (Vec<ACLStringConstraint>                        ),
	fld_tail  (Vec<ACLStringConstraint>                        ),
	fld_end
};

type ACLRule {
	Vec<ACLNameConstraint> name_constraints;
	Vec<ACLNameModifier>   name_modifiers;
	Vec<ACLTypeConstraint> rrtype_constraints;
	Vec<ACLTypeModifier>   rrtype_modifiers;
	Vec<ACLFieldRule>      fields_rules;
};

type ACL {
	Vec<ACLRule>;
};

type TreeShapedACL {
	Map<Label,TreeShapedACL> sub2acl;
	Map<RRType,Rc<ACL>>      rr2acl;
};


