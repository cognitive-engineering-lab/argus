import A

extension U: Comparable {
    func <(lhs: Self, rhs: Self) -> Bool {
        switch (lhs, rhs) {
            case (.y, .x):
                return true
            default:
                return false
        }
    }
}

public func insertp(e: U, es: Set<U>) -> Void {
    es.insert(e)
    return
}
